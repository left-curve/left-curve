use {
    crate::{TestAccounts, TestSuiteWithIndexer},
    dango_genesis::Contracts,
    dango_oracle::PYTH_PRICES,
    dango_order_book::{Dimensionless, OrderKind, Quantity, TimeInForce, UsdPrice},
    dango_types::{
        constants::usdc,
        oracle::{self, Precision, PrecisionlessPrice, PriceSource},
        perps,
    },
    grug::{
        Addr, BorshSerExt, Coins, Denom, NumberConst, ResultExt, Storage, Timestamp, Udec128,
        Uint128, btree_map, concat,
    },
    grug_app::CONTRACT_NAMESPACE,
    pyth_types::{Channel, PythId},
    std::collections::BTreeMap,
};

pub fn pair_id() -> Denom {
    "perp/ethusd".parse().unwrap()
}

/// Specification for a single oracle price entry used in tests.
///
/// `pyth_id` is the synthetic Pyth Lazer feed ID under which the price is
/// stored. Tests can use any `u32`; the value only has to be unique across
/// entries within the same test environment.
pub struct OracleTestEntry {
    pub pyth_id: PythId,
    pub precision: Precision,
    pub humanized_price: Udec128,
    pub timestamp: Timestamp,
}

/// Write a `PrecisionlessPrice` directly into the oracle contract's
/// `PYTH_PRICES` storage map.
///
/// This bypasses `ExecuteMsg::FeedPrices`, which would otherwise require a
/// valid Pyth Lazer ECDSA signature — impractical to satisfy in tests, since
/// the trusted signer's private key is not available to us.
pub fn write_pyth_price_raw(
    storage: &mut dyn Storage,
    oracle: Addr,
    pyth_id: PythId,
    price: &PrecisionlessPrice,
) {
    let map_key = PYTH_PRICES.path(pyth_id).storage_key().to_vec();
    let contract_prefix = concat(CONTRACT_NAMESPACE, oracle.as_ref());
    let full_key = concat(&contract_prefix, &map_key);
    let value_bytes = price.to_borsh_vec().unwrap();
    storage.write(&full_key, &value_bytes);
}

/// Common setup: register oracle prices + deposit margin for user1 and user2.
pub async fn setup_perps_env(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    eth_price: u128,
    margin_per_user: u128,
) {
    let entries = btree_map! {
        usdc::DENOM.clone() => OracleTestEntry {
            pyth_id: 1,
            precision: usdc::DECIMAL as Precision,
            humanized_price: Udec128::ONE,
            timestamp: Timestamp::from_nanos(u128::MAX),
        },
        pair_id() => OracleTestEntry {
            pyth_id: 2,
            precision: 0,
            humanized_price: Udec128::new(eth_price),
            timestamp: Timestamp::from_nanos(u128::MAX),
        },
    };

    let price_sources: BTreeMap<Denom, PriceSource> = entries
        .iter()
        .map(|(denom, e)| {
            (denom.clone(), PriceSource {
                id: e.pyth_id,
                channel: Channel::RealTime,
                precision: e.precision,
            })
        })
        .collect();

    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(price_sources),
            Coins::new(),
        )
        .await
        .should_succeed();

    suite.app.db.with_state_storage_mut(|storage| {
        for entry in entries.values() {
            let price = PrecisionlessPrice::new(entry.humanized_price, entry.timestamp);
            write_pyth_price_raw(storage, contracts.oracle, entry.pyth_id, &price);
        }
    });

    for account in [&mut accounts.user1, &mut accounts.user2] {
        let amount = Uint128::new(margin_per_user * 1_000_000);
        suite
            .execute(
                account,
                contracts.perps,
                &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
                Coins::one(usdc::DENOM.clone(), amount).unwrap(),
            )
            .await
            .should_succeed();
    }
}

/// Place a limit ask (user2) then a market buy (user1) to produce an
/// `OrderFilled` at the given price and size.
pub async fn create_perps_fill(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    pair_id: &Denom,
    price: u128,
    size: u128,
) {
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair_id.clone(),
                size: Quantity::new_int(-(size as i128)),
                kind: OrderKind::Limit {
                    limit_price: UsdPrice::new_int(price as i128),
                    time_in_force: TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .await
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair_id.clone(),
                size: Quantity::new_int(size as i128),
                kind: OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .await
        .should_succeed();
}
