use {
    crate::state::{PAIR_IDS, PAIR_PARAMS, PAIR_STATES, PARAM},
    anyhow::ensure,
    dango_types::perps::{PairParam, PairState, Param},
    grug::{Denom, GENESIS_SENDER, MutableCtx, QuerierExt, Response},
    std::collections::BTreeMap,
};

/// Update global and per-pair parameters.
/// Callable by the chain owner or `GENESIS_SENDER` (during instantiation).
pub fn configure(
    ctx: MutableCtx,
    param: Param,
    pair_params: BTreeMap<Denom, PairParam>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()? || ctx.sender == GENESIS_SENDER,
        "You don't have the right, O you don't have the right"
    );

    PARAM.save(ctx.storage, &param)?;

    for (pair_id, pair_param) in &pair_params {
        PAIR_PARAMS.save(ctx.storage, pair_id, pair_param)?;

        if !PAIR_STATES.has(ctx.storage, pair_id) {
            PAIR_STATES.save(ctx.storage, pair_id, &PairState::default())?;
        }
    }

    PAIR_IDS.save(ctx.storage, &pair_params.into_keys().collect())?;

    Ok(Response::new())
}
