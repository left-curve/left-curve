use {
    crate::NoCachePairQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{UsdValue, perps::UserState},
};

/// Compute the margin required to maintain the user's open positions, in USD.
///
/// For each position, the maintenance margin is:
///
/// ```plain
/// |position.size| * oracle_price * maintenance_margin_ratio
/// ```
///
/// The total maintenance margin is the sum of that of all positions.
///
/// The maintenance margin acts as the liquidation trigger. If a user's collateral
/// value falls below the maintenance margin, he becomes eligible for liquidation.
pub fn compute_maintenance_margin(
    user_state: &UserState,
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<UsdValue> {
    let mut total = UsdValue::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_param = pair_querier.query_pair_param(pair_id)?;

        let margin = position
            .size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.maintenance_margin_ratio)?;

        total = total.checked_add(margin)?;
    }

    Ok(total)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            HumanAmount, Ratio, UsdPrice,
            constants::{btc, eth},
            oracle::PrecisionedPrice,
            perps::{PairParam, Position},
        },
        grug::{Timestamp, Udec128, btree_map, hash_map},
        std::collections::HashMap,
        test_case::test_case,
    };

    #[test]
    fn no_positions() {
        let user_state = UserState::default();
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_maintenance_margin(&user_state, &pair_querier, &mut oracle_querier).unwrap(),
            UsdValue::ZERO,
        );
    }

    // size = 10 or -10, oracle = $2000, mmr = 5%
    // margin = |size| * 2000 * 0.05 = $1000
    #[test_case( 10 ; "long position")]
    #[test_case(-10 ; "short position")]
    fn single_position_works(size: i128) {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(size),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                maintenance_margin_ratio: Ratio::new_permille(50),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_maintenance_margin(&user_state, &pair_querier, &mut oracle_querier).unwrap(),
            UsdValue::new(1000),
        );
    }

    // ETH: |10| * $2000 * 5% = $1000
    // BTC: |1|  * $50000 * 3% = $1500
    // Total = $2500
    #[test]
    fn multiple_positions() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
                btc::DENOM.clone() => Position {
                    size: HumanAmount::new(-1),
                    entry_price: UsdPrice::new_int(50000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                maintenance_margin_ratio: Ratio::new_permille(50),
                ..Default::default()
            },
            btc::DENOM.clone() => PairParam {
                maintenance_margin_ratio: Ratio::new_permille(30),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
            btc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        assert_eq!(
            compute_maintenance_margin(&user_state, &pair_querier, &mut oracle_querier).unwrap(),
            UsdValue::new(2500),
        );
    }
}
