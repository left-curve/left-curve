use {
    dango_genesis::Contracts,
    dango_order_book::{Dimensionless, Quantity, UsdPrice},
    dango_testing::perps::{OracleTestEntry, pair_id, write_pyth_price_raw},
    dango_types::{
        constants::usdc,
        oracle::{self, Precision, PrecisionlessPrice, PriceSource},
        perps::{PairParam, Param, RateSchedule},
    },
    grug::{Coins, Duration, NumberConst, ResultExt, Timestamp, Udec128, btree_map},
    pyth_types::Channel,
    std::collections::BTreeMap,
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
        taker_fee_rates: RateSchedule {
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
///
/// Registers Pyth price sources via ExecuteMsg, then writes the prices
/// directly into `PYTH_PRICES` storage to bypass the Pyth Lazer signature
/// verification path.
pub async fn register_oracle_prices(
    suite: &mut dango_testing::TestSuite<grug_app::NaiveProposalPreparer>,
    accounts: &mut dango_testing::TestAccounts,
    contracts: &Contracts,
    eth_price: u128,
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

    let price_sources: BTreeMap<_, PriceSource> = entries
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
}
