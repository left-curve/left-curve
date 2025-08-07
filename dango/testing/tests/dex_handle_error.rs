use {
    dango_dex::INCOMING_ORDERS,
    dango_genesis::Contracts,
    dango_testing::setup_test_naive,
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateLimitOrderRequest, Direction, LimitOrder, OrderId, QueryPausedRequest},
    },
    grug::{
        Addr, Addressable, ContractBuilder, ContractWrapper, Empty, HashExt, Message, NonEmpty,
        NonZero, QuerierExt, Response, ResultExt, Signer, StdResult, StorageQuerier, SudoCtx,
        Udec128_6, Udec128_24, Uint128, coins,
    },
    test_case::test_case,
};

mod bugged_dex {
    use {
        anyhow::bail,
        dango_types::dex,
        grug::{MutableCtx, Response},
    };

    pub fn bugged_execute(ctx: MutableCtx, msg: dex::ExecuteMsg) -> anyhow::Result<Response> {
        match msg {
            dex::ExecuteMsg::Callback(_) => {
                bail!("BANG💥");
            },
            msg => dango_dex::execute(ctx, msg),
        }
    }
}

mod bugged_bank {
    use {
        anyhow::bail,
        grug::{BankMsg, Response, SudoCtx},
    };

    pub fn bugged_bank_execute(_ctx: SudoCtx, _msg: BankMsg) -> anyhow::Result<Response> {
        bail!("BOOM💥");
    }
}

mod bugged_taxman {
    use {
        anyhow::bail,
        dango_types::taxman,
        grug::{MutableCtx, Response},
    };

    pub fn bugged_execute(_ctx: MutableCtx, _msg: taxman::ExecuteMsg) -> anyhow::Result<Response> {
        bail!("KABOOM💥");
    }
}

fn do_nothing(_ctx: SudoCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[test_case(
    |contracts| {
        let bugged_dex_code = ContractBuilder::new(Box::new(dango_dex::instantiate))
            .with_cron_execute(Box::new(dango_dex::cron_execute))
            .with_reply(Box::new(dango_dex::reply))
            .with_query(Box::new(dango_dex::query))
            .with_execute(Box::new(bugged_dex::bugged_execute))
            .with_migrate(Box::new(do_nothing))
            .build();
        (contracts.dex, bugged_dex_code)
    };
    "intentional error in dex execute"
)]
#[test_case(
    |contracts| {
        let bugged_bank_code = ContractBuilder::new(Box::new(dango_bank::instantiate))
            .with_bank_execute(Box::new(bugged_bank::bugged_bank_execute))
            .with_migrate(Box::new(do_nothing))
            .build();
        (contracts.bank, bugged_bank_code)
    };
    "intentional error in bank transfer"
)]
#[test_case(
    |contracts| {
        let bugged_taxman_code = ContractBuilder::new(Box::new(dango_taxman::instantiate))
            .with_withhold_fee(Box::new(dango_taxman::withhold_fee))
            .with_finalize_fee(Box::new(dango_taxman::finalize_fee))
            .with_execute(Box::new(bugged_taxman::bugged_execute))
            .with_migrate(Box::new(do_nothing))
            .build();
        (contracts.taxman, bugged_taxman_code)
    };
    "intentional error in taxman fee payment"
)]
fn handling_error_in_auction(f: fn(&Contracts) -> (Addr, ContractWrapper)) {
    // grug::setup_tracing_subscriber(tracing::Level::INFO); // uncomment to see tracing logs

    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let (contract_to_migrate, bugged_code) = f(&contracts);
    let bugged_code_hash = bugged_code.to_bytes().hash256();

    // Send a tx with 3 actions:
    // 1. submit a BUY order
    // 2. submit a SELL order
    // 3. migrate a contract (either the DEX, bank, or taxman) to a bugged version
    //    that intentionally fails.
    // The two orders will match and be fulfilled under normal circumstances,
    // but the auction fails, we ensure that the orders are NOT fulfilled and
    // trading is automatically halted.
    let tx = accounts
        .owner
        .sign_transaction(
            NonEmpty::new_unchecked(vec![
                Message::execute(
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates_market: vec![],
                        creates_limit: vec![
                            CreateLimitOrderRequest {
                                base_denom: dango::DENOM.clone(),
                                quote_denom: usdc::DENOM.clone(),
                                direction: Direction::Bid,
                                amount: NonZero::new_unchecked(Uint128::new(3)),
                                price: NonZero::new_unchecked(Udec128_24::new(100)),
                            },
                            CreateLimitOrderRequest {
                                base_denom: dango::DENOM.clone(),
                                quote_denom: usdc::DENOM.clone(),
                                direction: Direction::Ask,
                                amount: NonZero::new_unchecked(Uint128::new(3)),
                                price: NonZero::new_unchecked(Udec128_24::new(100)),
                            },
                        ],
                        cancels: None,
                    },
                    coins! {
                        dango::DENOM.clone() => 3,
                        usdc::DENOM.clone() => 300,
                    },
                )
                .unwrap(),
                Message::upload(bugged_code),
                Message::migrate(contract_to_migrate, bugged_code_hash, &Empty {}).unwrap(),
            ]),
            &suite.chain_id,
            100_000_000,
        )
        .unwrap();

    suite.make_block(vec![tx]);

    // Ensure trading is halted.
    suite
        .query_wasm_smart(contracts.dex, QueryPausedRequest {})
        .should_succeed_and_equal(true);

    // Ensure the two limit orders still exist the DEX as "incoming orders".
    suite
        .query_wasm_path(
            contracts.dex,
            &INCOMING_ORDERS.path((accounts.owner.address(), OrderId::new(!1))),
        )
        .should_succeed_and_equal((
            (
                (dango::DENOM.clone(), usdc::DENOM.clone()),
                Direction::Bid,
                Udec128_24::new(100),
                OrderId::new(!1),
            ),
            LimitOrder {
                user: accounts.owner.address(),
                id: OrderId::new(!1),
                price: Udec128_24::new(100),
                amount: Uint128::new(3),
                remaining: Udec128_6::new(3),
                created_at_block_height: 1,
            },
        ));
    suite
        .query_wasm_path(
            contracts.dex,
            &INCOMING_ORDERS.path((accounts.owner.address(), OrderId::new(2))),
        )
        .should_succeed_and_equal((
            (
                (dango::DENOM.clone(), usdc::DENOM.clone()),
                Direction::Ask,
                Udec128_24::new(100),
                OrderId::new(2),
            ),
            LimitOrder {
                user: accounts.owner.address(),
                id: OrderId::new(2),
                price: Udec128_24::new(100),
                amount: Uint128::new(3),
                remaining: Udec128_6::new(3),
                created_at_block_height: 1,
            },
        ));
}
