use {
    dango_genesis::Contracts,
    dango_testing::perps::pair_id,
    dango_types::{
        Dimensionless, Quantity, UsdPrice,
        constants::usdc,
        oracle::{self, PriceSource},
        perps::{self, PairParam, Param},
    },
    grug::{Coins, Duration, NumberConst, ResultExt, Timestamp, Udec128, btree_map},
};

mod adl_bug_reproduction;
mod batch_update_orders;
mod client_order_id;
mod conditional_orders;
mod liquidation;
mod price_band;
mod referral;
mod trading;
mod vault;
mod vault_snapshots;
mod vault_withdrawal_health;

/// Return the genesis-default global params (mirrors `PerpsOption::preset_test()`).
pub fn default_param() -> Param {
    Param {
        taker_fee_rates: perps::RateSchedule {
            base: Dimensionless::new_permille(1), // 0.1%
            ..Default::default()
        },
        protocol_fee_rate: Dimensionless::ZERO,
        liquidation_fee_rate: Dimensionless::new_permille(10), // 1%
        vault_cooldown_period: Duration::from_days(1),
        max_unlocks: 10,
        max_open_orders: 100,
        funding_period: Duration::from_hours(1),
        max_action_batch_size: 5,
        ..Default::default()
    }
}

/// Return the genesis-default pair params (mirrors `PerpsOption::preset_test()`).
pub fn default_pair_param() -> PairParam {
    PairParam {
        initial_margin_ratio: Dimensionless::new_permille(100), // 10%
        maintenance_margin_ratio: Dimensionless::new_permille(50), // 5%
        tick_size: UsdPrice::new_int(1),
        max_abs_oi: Quantity::new_int(1_000_000),
        ..PairParam::new_mock()
    }
}

/// Register fixed oracle prices for the perps pair and settlement currency.
pub fn register_oracle_prices(
    suite: &mut dango_testing::TestSuite<grug_app::NaiveProposalPreparer>,
    accounts: &mut dango_testing::TestAccounts,
    contracts: &Contracts,
    eth_price: u128,
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
}
