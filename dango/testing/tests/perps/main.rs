use {
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestSuite, perps::pair_id},
    dango_types::{
        Dimensionless, Quantity, UsdPrice,
        constants::usdc,
        oracle::{self, PriceSource},
        perps::{self, PairParam, Param},
    },
    grug::{
        Binary, ByteArray, Coins, Duration, NonEmpty, NumberConst, ResultExt, Timestamp, Udec128,
        btree_map,
    },
    grug_app::NaiveProposalPreparer,
    pyth_types::LeEcdsaMessage,
    std::str::FromStr,
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

/// Trigger a vault refresh via the oracle's `FeedPrices` path.
///
/// `refresh_orders` only accepts calls from the oracle contract (or the perps
/// contract itself). This helper feeds a valid Pyth message to the oracle,
/// which dispatches the `Refresh` submessage with the oracle as sender.
/// The Fixed price sources set by [`register_oracle_prices`] are unaffected —
/// only the Pyth price store is touched.
pub fn refresh_vault_orders(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) {
    // A valid Pyth Lazer message signed by the genesis-trusted signer.
    // The feed IDs inside don't correspond to any registered Fixed price
    // source, so feeding this message only triggers the Refresh submessage
    // without altering the prices used by perps.
    let message = LeEcdsaMessage {
        payload: Binary::from_str(
            "ddPHkyAnhCsRTAYAAQICAAAAAgDLzMJzLwAAAAT4/wcAAAACAPnb9QUAAAAABPj/",
        )
        .unwrap(),
        signature: ByteArray::from_str(
            "HJt9BJHEBuX0VhWDIjldnfwIYO9ufenGCVTMhQUwxhoYiX+TVDSqbNdQpXsRilNrS9Z7q/ET8obCBM9c97DmcQ==",
        )
        .unwrap(),
        recovery_id: 1,
    };

    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message])),
            Coins::new(),
        )
        .should_succeed();
}
