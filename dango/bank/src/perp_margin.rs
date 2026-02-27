use {
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_perps::{NoCachePerpQuerier, core::compute_available_margin},
    dango_types::{
        Quantity,
        perps::{UserState, settlement_currency},
    },
    grug::Uint128,
};

/// Ensure that a settlement currency transfer does not exceed the sender's
/// available margin in the perps contract.
pub(crate) fn check_perps_margin(
    user_state: &UserState,
    balance: Uint128,
    transfer_amount: Uint128,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<()> {
    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    let collateral_value = Quantity::from_base(balance, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    let available_margin = compute_available_margin(
        collateral_value,
        user_state,
        perp_querier,
        oracle_querier,
        user_state.reserved_margin,
    )?;

    let available_in_settlement = available_margin.checked_div(settlement_currency_price)?;
    let available_base = available_in_settlement.into_base_floor(settlement_currency::DECIMAL)?;

    ensure!(
        transfer_amount <= available_base,
        "transfer of {transfer_amount} exceeds available margin of {available_base}"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            Dimensionless, FundingPerUnit, UsdPrice,
            constants::eth,
            oracle::PrecisionedPrice,
            perps::{PairParam, PairState, Position},
        },
        grug::{Timestamp, Udec128, btree_map, hash_map},
        std::collections::HashMap,
    };

    fn usdc_price() -> PrecisionedPrice {
        PrecisionedPrice::new(Udec128::new_percent(100), Timestamp::from_seconds(0), 6)
    }

    fn eth_price(dollars: u64) -> PrecisionedPrice {
        PrecisionedPrice::new(
            Udec128::new_percent(u128::from(dollars) * 100),
            Timestamp::from_seconds(0),
            18,
        )
    }

    /// ETH long 10 @ entry=$2000, IMR=10%, no funding, no reserved margin.
    fn eth_long_setup(
        oracle_eth_price: u64,
    ) -> (
        UserState,
        NoCachePerpQuerier<'static>,
        OracleQuerier<'static>,
    ) {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
        let oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price(),
            eth::DENOM.clone() => eth_price(oracle_eth_price),
        });
        (user_state, perp_querier, oracle_querier)
    }

    #[test]
    fn no_positions_passes() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price(),
        });

        check_perps_margin(
            &user_state,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap();
    }

    // oracle=2500, balance=10,000 USDC
    // equity = 10000 + 5000 = 15000, used = 2500, available = 12500
    // available_base = 12,500,000,000
    #[test]
    fn sufficient_margin_passes() {
        let (user_state, perp_querier, mut oracle_querier) = eth_long_setup(2500);

        check_perps_margin(
            &user_state,
            Uint128::new(10_000_000_000),
            Uint128::new(5_000_000_000),
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap();
    }

    #[test]
    fn exact_margin_passes() {
        let (user_state, perp_querier, mut oracle_querier) = eth_long_setup(2500);

        check_perps_margin(
            &user_state,
            Uint128::new(10_000_000_000),
            Uint128::new(12_500_000_000),
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap();
    }

    #[test]
    fn exceeds_margin_fails() {
        let (user_state, perp_querier, mut oracle_querier) = eth_long_setup(2500);

        let result = check_perps_margin(
            &user_state,
            Uint128::new(10_000_000_000),
            Uint128::new(12_500_000_001),
            &perp_querier,
            &mut oracle_querier,
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds available margin"),
        );
    }

    // oracle=1500, balance=100 USDC
    // pnl = -5000, equity = -4900, used = 1500, clamped to 0
    #[test]
    fn zero_available_margin_fails() {
        let (user_state, perp_querier, mut oracle_querier) = eth_long_setup(1500);

        let result = check_perps_margin(
            &user_state,
            Uint128::new(100_000_000),
            Uint128::new(1),
            &perp_querier,
            &mut oracle_querier,
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds available margin"),
        );
    }
}
