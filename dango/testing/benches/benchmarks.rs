use {
    criterion::{
        AxisScale, BatchSize, Criterion, PlotConfiguration, criterion_group, criterion_main,
    },
    dango_dex::ORDERS,
    dango_genesis::{Codes, Contracts},
    dango_testing::{TestAccounts, TestSuite, setup_benchmark_hybrid, setup_benchmark_wasm},
    dango_types::{
        account::single,
        account_factory::{self, AccountParams, Salt},
        constants::{btc, dango, usdc},
        dex::{Direction, Order, OrderId, Price, TimeInForce},
    },
    grug::{
        Addr, Binary, Buffer, Coins, Denom, HashExt, JsonSerExt, Message, NonEmpty, ResultExt,
        Shared, Storage, Tx, Udec128_6, Uint128, coins,
    },
    grug_app::{AppError, CONTRACT_NAMESPACE, Db, ProposalPreparer, StorageProvider, Vm},
    grug_db_disk_lite::DiskDbLite,
    rand::{Rng, distributions::Alphanumeric},
    std::{str::FromStr, time::Duration},
    temp_rocksdb::TempDataDir,
};

const MEASUREMENT_TIME: Duration = Duration::from_secs(90);

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn do_send<T, PP, DB, VM>(
    suite: &mut TestSuite<PP, DB, VM>,
    mut accounts: TestAccounts,
    codes: Codes<T>,
    contracts: Contracts,
) -> Vec<Tx>
where
    T: Into<Binary>,
    PP: ProposalPreparer,
    DB: Db,
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
{
    // Deploy 200 accounts.
    // The first 100 will be senders; the second 100 will be receivers.
    // For convenience, all accounts are owned by the relayer.
    let msgs = (0..200)
        .map(|i| {
            Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterAccount {
                    params: AccountParams::Spot(single::Params::new(
                        accounts.user1.username.clone(),
                    )),
                },
                if i < 100 {
                    coins! { usdc::DENOM.clone() => 100_000_000 }
                } else {
                    Coins::new()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    // In experience, this costs ~34M gas.
    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    // Make a block that contains 100 transactions.
    // The i-th transaction is the i-th sender sending coins to the i-receiver.
    let code_account_spot = codes.account_spot.into().hash256();
    (0..100)
        .map(|i| {
            // Predict the sender address.
            // During genesis we created 3 accounts, so offset i by 3.
            let sender = Addr::derive(
                contracts.account_factory,
                code_account_spot,
                Salt { index: i + 3 }.into_bytes().as_slice(),
            );

            // Predict the receiver address.
            let receiver = Addr::derive(
                contracts.account_factory,
                code_account_spot,
                Salt { index: i + 103 }.into_bytes().as_slice(),
            );

            // Sign the transaction.
            let msg = Message::transfer(receiver, coins! { usdc::DENOM.clone() => 123 }).unwrap();

            let (data, credential) = accounts
                .owner
                .sign_transaction_with_nonce(
                    sender,
                    NonEmpty::new_unchecked(vec![msg.clone()]),
                    &suite.chain_id,
                    2_000_000,
                    0,
                    None,
                )
                .unwrap();

            Tx {
                sender,
                gas_limit: 2_000_000,
                msgs: NonEmpty::new_unchecked(vec![msg]),
                data: data.to_json_value().unwrap(),
                credential: credential.to_json_value().unwrap(),
            }
        })
        .collect()
}

/// Measure how many token transfers can be processed in a second.
///
/// We do this by making a single block that contains 100 transactions, each tx
/// containing one `Message::Transfer`.
fn sends(c: &mut Criterion) {
    let mut group = c.benchmark_group("sends");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Linear));
    group.measurement_time(MEASUREMENT_TIME);

    group.bench_function("send-wasm", |b| {
        b.iter_batched(
            || {
                // Create a random folder for this iteration.
                let dir = TempDataDir::new(&format!("__dango_bench_sends_{}", random_string(8)));
                let (mut suite, accounts, codes, contracts, _) = setup_benchmark_wasm(&dir, 100);

                let txs = do_send(&mut suite, accounts, codes, contracts);

                // Note: `dir` must be passed to the routine, so that it's alive
                // until the end of this iteration.
                (dir, suite, txs)
            },
            |(_dir, mut suite, txs)| {
                suite
                    .make_block(txs)
                    .block_outcome
                    .tx_outcomes
                    .into_iter()
                    .all(|outcome| outcome.result.is_ok());
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("send-hybrid", |b| {
        b.iter_batched(
            || {
                // Create a random folder for this iteration.
                let dir = TempDataDir::new(&format!("__dango_bench_sends_{}", random_string(8)));
                let (mut suite, accounts, codes, contracts, _) = setup_benchmark_hybrid(&dir, 100);

                let txs = do_send(&mut suite, accounts, codes, contracts);

                // Note: `dir` must be passed to the routine, so that it's alive
                // until the end of this iteration.
                (dir, suite, txs)
            },
            |(_dir, mut suite, txs)| {
                suite
                    .make_block(txs)
                    .block_outcome
                    .tx_outcomes
                    .into_iter()
                    .all(|outcome| outcome.result.is_ok());
            },
            BatchSize::SmallInput,
        );
    });
}

fn setup_storage() -> (TempDataDir, StorageProvider) {
    let dir = TempDataDir::new(&format!("__dango_bench_storage_{}", random_string(8)));
    let db = DiskDbLite::open(&dir).unwrap();
    let buffer = Buffer::new(db.state_storage(None).unwrap(), None);
    let shared = Shared::new(buffer);
    let mut storage = StorageProvider::new(Box::new(shared.clone()), &[
        CONTRACT_NAMESPACE,
        &Addr::mock(1),
    ]);

    let mut order_id: u64 = 0;

    let data = [
        (&dango::DENOM, Direction::Bid, "1.00"),
        (&dango::DENOM, Direction::Bid, "1.01"),
        (&dango::DENOM, Direction::Bid, "1.02"),
        (&dango::DENOM, Direction::Bid, "1.03"),
        (&dango::DENOM, Direction::Ask, "1.00"),
        (&dango::DENOM, Direction::Ask, "1.01"),
        (&dango::DENOM, Direction::Ask, "1.02"),
        (&dango::DENOM, Direction::Ask, "1.03"),
        (&btc::DENOM, Direction::Bid, "10234.1234"),
        (&btc::DENOM, Direction::Bid, "10235.1235"),
        (&btc::DENOM, Direction::Bid, "10236.1236"),
        (&btc::DENOM, Direction::Ask, "10234.1234"),
        (&btc::DENOM, Direction::Ask, "10235.1235"),
        (&btc::DENOM, Direction::Ask, "10236.1236"),
    ];

    for (base, direction, price) in data {
        let price = Price::from_str(price).unwrap();
        order_id += 1;

        let id = OrderId::new(if direction == Direction::Bid {
            !order_id
        } else {
            order_id
        });

        ORDERS
            .save(
                &mut storage,
                (((*base).clone(), usdc::DENOM.clone()), direction, price, id),
                &Order {
                    user: Addr::mock(2),
                    id,
                    direction,
                    time_in_force: TimeInForce::GoodTilCanceled,
                    price,
                    amount: Uint128::new(100),
                    remaining: Udec128_6::new(100),
                    created_at_block_height: Some(123),
                },
            )
            .unwrap();
    }

    drop(storage);

    let (_, batch) = shared.disassemble().disassemble();

    assert_eq!(batch.len(), data.len() * 4);
    db.flush_and_commit(batch).unwrap();

    let buffer = Buffer::new(db.state_storage(None).unwrap(), None);
    let shared = Shared::new(buffer);
    let storage = StorageProvider::new(Box::new(shared), &[CONTRACT_NAMESPACE, &Addr::mock(1)]);

    (dir, storage)
}

fn routine_storage<S: Storage>(
    (_dir, storage): (TempDataDir, S),
    order: grug::Order,
    base_denom: Denom,
) {
    let direction = if order == grug::Order::Ascending {
        Direction::Ask
    } else {
        Direction::Bid
    };

    let mut bid_iter = ORDERS
        .prefix((base_denom.clone(), usdc::DENOM.clone()))
        .append(direction)
        .values(&storage, None, None, order);

    assert!(bid_iter.next().is_some());
}

fn storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Linear));
    // group.sample_size(100)
    group.measurement_time(std::time::Duration::from_secs(2)); // default 5s
    group.warm_up_time(std::time::Duration::from_secs(1));

    group.bench_function("bid-dango", |b| {
        b.iter_batched(
            setup_storage,
            |a| routine_storage(a, grug::Order::Descending, dango::DENOM.clone()),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("ask-dango", |b| {
        b.iter_batched(
            setup_storage,
            |a| routine_storage(a, grug::Order::Ascending, dango::DENOM.clone()),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("bid-btc", |b| {
        b.iter_batched(
            setup_storage,
            |a| routine_storage(a, grug::Order::Descending, btc::DENOM.clone()),
            BatchSize::SmallInput,
        );
    });

    group.bench_function("ask-btc", |b| {
        b.iter_batched(
            setup_storage,
            |a| routine_storage(a, grug::Order::Ascending, btc::DENOM.clone()),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, sends, storage);

criterion_main!(benches);
