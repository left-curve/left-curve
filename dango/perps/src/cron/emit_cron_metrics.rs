use {
    crate::{
        core::compute_user_equity,
        querier::NoCachePerpQuerier,
        state::{STATE, USER_STATES},
    },
    dango_oracle::OracleQuerier,
    grug::{Addr, Inner, Storage},
    std::time::Instant,
};

/// Emit metrics gauges and histograms captured during cron execution.
///
/// Loads vault state to report equity, margin, positions, insurance fund,
/// and treasury gauges, then records the cron duration histogram.
pub fn emit_cron_metrics(
    storage: &dyn Storage,
    contract: Addr,
    oracle_querier: &mut OracleQuerier,
    start: Instant,
) -> anyhow::Result<()> {
    let state = STATE.load(storage)?;
    let vault_state = USER_STATES.may_load(storage, contract)?.unwrap_or_default();
    let perp_querier = NoCachePerpQuerier::new_local(storage);

    if let Ok(vault_equity) = compute_user_equity(oracle_querier, &perp_querier, &vault_state) {
        metrics::gauge!(crate::metrics::LABEL_VAULT_EQUITY).set(vault_equity.to_f64());
    }

    for (pair_id, position) in &vault_state.positions {
        metrics::gauge!(
            crate::metrics::LABEL_VAULT_POSITION,
            "pair_id" => pair_id.to_string()
        )
        .set(position.size.to_f64());
    }

    metrics::gauge!(crate::metrics::LABEL_VAULT_MARGIN).set(vault_state.margin.to_f64());

    metrics::gauge!(crate::metrics::LABEL_VAULT_SHARE_SUPPLY)
        .set(state.vault_share_supply.into_inner() as f64);

    metrics::gauge!(crate::metrics::LABEL_INSURANCE_FUND).set(state.insurance_fund.to_f64());

    metrics::gauge!(crate::metrics::LABEL_TREASURY).set(state.treasury.to_f64());

    metrics::histogram!(crate::metrics::LABEL_DURATION_CRON).record(start.elapsed().as_secs_f64());

    Ok(())
}
