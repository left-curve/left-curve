use {
    crate::{TestAccounts, TestSuiteWithIndexer},
    dango_genesis::Contracts,
    dango_order_book::{Dimensionless, OrderKind, Quantity, TimeInForce, UsdPrice},
    dango_types::{
        constants::usdc,
        oracle::{self, PriceSource},
        perps,
    },
    grug::{Coins, Denom, NumberConst, ResultExt, Timestamp, Udec128, Uint128, btree_map},
};

pub fn pair_id() -> Denom {
    "perp/ethusd".parse().unwrap()
}

/// Common setup: register oracle prices + deposit margin for user1 and user2.
pub fn setup_perps_env(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    eth_price: u128,
    margin_per_user: u128,
) {
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: usdc::DECIMAL as u8,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
                pair_id() => PriceSource::Fixed {
                    humanized_price: Udec128::new(eth_price),
                    precision: 0,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    for account in [&mut accounts.user1, &mut accounts.user2] {
        let amount = Uint128::new(margin_per_user * 1_000_000);
        suite
            .execute(
                account,
                contracts.perps,
                &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
                Coins::one(usdc::DENOM.clone(), amount).unwrap(),
            )
            .should_succeed();
    }
}

/// Place a limit ask (user2) then a market buy (user1) to produce an
/// `OrderFilled` at the given price and size.
pub fn create_perps_fill(
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
        .should_succeed();
}
