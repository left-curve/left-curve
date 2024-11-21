use {
    crate::POSITIONS,
    dango_types::vesting::{ClaimablePosition, QueryMsg},
    grug::{Addr, Bound, ImmutableCtx, Json, JsonSerExt, StdResult},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Position { idx } => position(ctx, idx)?.to_json_value(),
        QueryMsg::Positions { start_after, limit } => {
            positions(ctx, start_after, limit)?.to_json_value()
        },
        QueryMsg::PositionsByUser {
            user,
            start_after,
            limit,
        } => positions_by_user(ctx, user, start_after, limit)?.to_json_value(),
    }
}

fn position(ctx: ImmutableCtx, idx: u64) -> StdResult<ClaimablePosition> {
    POSITIONS
        .load(ctx.storage, idx)
        .map(|val| val.with_claimable_amount(ctx.block.timestamp))
}

fn positions(
    ctx: ImmutableCtx,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<u64, ClaimablePosition>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;
    POSITIONS
        .range(ctx.storage, start, None, grug::Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(k, v)| (k, v.with_claimable_amount(ctx.block.timestamp))))
        .collect()
}

fn positions_by_user(
    ctx: ImmutableCtx,
    user: Addr,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<u64, ClaimablePosition>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;
    POSITIONS
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, start, None, grug::Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(k, v)| (k, v.with_claimable_amount(ctx.block.timestamp))))
        .collect()
}
