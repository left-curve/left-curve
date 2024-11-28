use {
    crate::{CONFIG, POSITIONS},
    dango_types::vesting::{ClaimablePosition, PositionIndex, QueryMsg},
    grug::{Addr, Bound, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Position { idx } => {
            let res = query_position(ctx, idx)?;
            res.to_json_value()
        },
        QueryMsg::Positions { start_after, limit } => {
            let res = query_positions(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::PositionsByUser {
            user,
            start_after,
            limit,
        } => {
            let res = query_positions_by_user(ctx, user, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_position(ctx: ImmutableCtx, idx: PositionIndex) -> StdResult<ClaimablePosition> {
    let unlocking_schedule = CONFIG.load(ctx.storage)?.unlocking_schedule;

    POSITIONS
        .load(ctx.storage, idx)
        .map(|val| val.with_claimable_amount(ctx.block.timestamp, &unlocking_schedule))
}

fn query_positions(
    ctx: ImmutableCtx,
    start_after: Option<PositionIndex>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<PositionIndex, ClaimablePosition>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    let unlocking_schedule = CONFIG.load(ctx.storage)?.unlocking_schedule;

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

fn query_positions_by_user(
    ctx: ImmutableCtx,
    user: Addr,
    start_after: Option<PositionIndex>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<PositionIndex, ClaimablePosition>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    let unlocking_schedule = CONFIG.load(ctx.storage)?.unlocking_schedule;

    POSITIONS
        .idx
        .user
        .prefix(user)
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
