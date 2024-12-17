use {
    crate::{CLIENTS, CLIENT_REGISTRY, RAW_CLIENT_STATES, RAW_CONSENSUS_STATES},
    dango_types::ibc::{
        client::Height,
        host::{Client, ClientId, ClientType, QueryMsg},
    },
    grug::{Addr, Binary, Bound, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::ClientImpl { client_type } => {
            let res = query_client_impl(ctx, client_type)?;
            res.to_json_value()
        },
        QueryMsg::ClientImpls { start_after, limit } => {
            let res = query_client_impls(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Client { client_id } => {
            let res = query_client(ctx, client_id)?;
            res.to_json_value()
        },
        QueryMsg::Clients { start_after, limit } => {
            let res = query_clients(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::ClientState { client_id } => {
            let res = query_client_state(ctx, client_id)?;
            res.to_json_value()
        },
        QueryMsg::ClientStates { start_after, limit } => {
            let res = query_client_states(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::ConsensusState { client_id, height } => {
            let res = query_consensus_state(ctx, client_id, height)?;
            res.to_json_value()
        },
        QueryMsg::ConsensusStates {
            client_id,
            start_after,
            limit,
        } => {
            let res = query_consensus_states(ctx, client_id, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_client_impl(ctx: ImmutableCtx, client_type: ClientType) -> StdResult<Addr> {
    CLIENT_REGISTRY.load(ctx.storage, client_type)
}

fn query_client_impls(
    ctx: ImmutableCtx,
    start_after: Option<ClientType>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<ClientType, Addr>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CLIENT_REGISTRY
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_client(ctx: ImmutableCtx, client_id: ClientId) -> StdResult<Client> {
    CLIENTS.load(ctx.storage, client_id)
}

fn query_clients(
    ctx: ImmutableCtx,
    start_after: Option<ClientId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<ClientId, Client>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    CLIENTS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_client_state(ctx: ImmutableCtx, client_id: ClientId) -> StdResult<Binary> {
    RAW_CLIENT_STATES.load(ctx.storage, client_id)
}

fn query_client_states(
    ctx: ImmutableCtx,
    start_after: Option<ClientId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<ClientId, Binary>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    RAW_CLIENT_STATES
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_consensus_state(
    ctx: ImmutableCtx,
    client_id: ClientId,
    height: Height,
) -> StdResult<Binary> {
    RAW_CONSENSUS_STATES.load(ctx.storage, (client_id, height))
}

fn query_consensus_states(
    ctx: ImmutableCtx,
    client_id: ClientId,
    start_after: Option<Height>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Height, Binary>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    RAW_CONSENSUS_STATES
        .prefix(client_id)
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}
