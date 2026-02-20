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
