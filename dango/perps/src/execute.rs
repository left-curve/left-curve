use {
    crate::{PAIR_PARAMS, PAIR_STATES, PARAM, STATE},
    dango_types::perps::{InstantiateMsg, PairState, State},
    grug::{MutableCtx, Response},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    PARAM.save(ctx.storage, &msg.param)?;
    STATE.save(ctx.storage, &State::default())?;

    for (pair_id, pair_param) in msg.pair_params {
        PAIR_PARAMS.save(ctx.storage, &pair_id, &pair_param)?;
        PAIR_STATES.save(ctx.storage, &pair_id, &PairState::new(ctx.block.timestamp))?;
    }

    Ok(Response::new())
}
