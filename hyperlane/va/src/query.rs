use {
    crate::{MAILBOX, STORAGE_LOCATIONS},
    grug::{
        Addr, Bound, HexByteArray, ImmutableCtx, Json, JsonSerExt, Order, StdResult, UniqueVec,
    },
    hyperlane_types::va::QueryMsg,
    std::collections::{BTreeMap, BTreeSet},
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Mailbox {} => {
            let res = query_mailbox(ctx)?;
            res.to_json_value()
        },
        QueryMsg::AnnouncedValidators { start_after, limit } => {
            let res = query_announced_validators(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::AnnouncedStorageLocations { validators } => {
            let res = query_announced_storage_locations(ctx, validators)?;
            res.to_json_value()
        },
    }
}

fn query_mailbox(ctx: ImmutableCtx) -> StdResult<Addr> {
    MAILBOX.load(ctx.storage)
}

fn query_announced_validators(
    ctx: ImmutableCtx,
    start_after: Option<HexByteArray<20>>,
    limit: Option<u32>,
) -> StdResult<BTreeSet<HexByteArray<20>>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    STORAGE_LOCATIONS
        .keys(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_announced_storage_locations(
    ctx: ImmutableCtx,
    validators: BTreeSet<HexByteArray<20>>,
) -> StdResult<BTreeMap<HexByteArray<20>, UniqueVec<String>>> {
    validators
        .into_iter()
        .map(|v| {
            let storage_locations = STORAGE_LOCATIONS.load(ctx.storage, v).unwrap_or_default();
            Ok((v, storage_locations))
        })
        .collect()
}
