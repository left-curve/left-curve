use {
    crate::{POSITIONS, UNLOCKING_SCHEDULE},
    dango_types::vesting::{ClaimablePosition, QueryMsg},
    grug::{Addr, Bound, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Position { user } => {
            let res = query_position(ctx, user)?;
            res.to_json_value()
        },
        QueryMsg::Positions { start_after, limit } => {
            let res = query_positions(ctx, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_position(ctx: ImmutableCtx, user: Addr) -> StdResult<ClaimablePosition> {
    let unlocking_schedule = UNLOCKING_SCHEDULE.load(ctx.storage)?;

    POSITIONS
        .load(ctx.storage, user)
        .map(|val| val.with_claimable_amount(ctx.block.timestamp, &unlocking_schedule))
}

fn query_positions(
    ctx: ImmutableCtx,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Addr, ClaimablePosition>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    let unlocking_schedule = UNLOCKING_SCHEDULE.load(ctx.storage)?;

    POSITIONS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(k, v)| {
                (
                    k,
                    v.with_claimable_amount(ctx.block.timestamp, &unlocking_schedule),
                )
            })
        })
        .collect()
}
