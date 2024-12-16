use {
    crate::CLIENT_IMPLS,
    dango_types::ibc::host::{ClientType, QueryMsg},
    grug::{Addr, Bound, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Client(client_type) => {
            let res = query_client(ctx, client_type)?;
            res.to_json_value()
        },
        QueryMsg::Clients { start_after, limit } => {
            let res = query_clients(ctx, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_client(ctx: ImmutableCtx, client_type: ClientType) -> StdResult<Addr> {
    CLIENT_IMPLS.load(ctx.storage, client_type)
}

fn query_clients(
    ctx: ImmutableCtx,
    start_after: Option<ClientType>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<ClientType, Addr>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CLIENT_IMPLS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}
