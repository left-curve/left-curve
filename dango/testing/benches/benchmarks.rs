use {
    criterion::{
        AxisScale, BatchSize, Criterion, PlotConfiguration, criterion_group, criterion_main,
    },
    dango_genesis::{Codes, Contracts},
    dango_testing::{TestAccounts, TestSuite, setup_benchmark_hybrid, setup_benchmark_wasm},
    dango_types::{
        account::single,
        account_factory::{self, AccountParams, Salt},
    },
    grug::{Addr, Binary, Coins, HashExt, JsonSerExt, Message, NonEmpty, ResultExt, Tx},
    grug_app::{AppError, Db, ProposalPreparer, Vm},
    grug_db_disk::TempDataDir,
    rand::{Rng, distributions::Alphanumeric},
    std::time::Duration,
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
                    Coins::one("uusdc", 100_000_000).unwrap()
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
            let msg = Message::transfer(receiver, Coins::one("uusdc", 123).unwrap()).unwrap();

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

criterion_group!(benches, sends);

criterion_main!(benches);
