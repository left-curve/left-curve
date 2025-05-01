use {
    crate::{MAILBOX, ROUTES},
    dango_types::warp::{QueryMsg, QueryRoutesPageParam, QueryRoutesResponseItem, Route},
    grug::{
        Addr, Bound, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult,
    },
    hyperlane_types::{
        mailbox::Domain,
        recipients::{RecipientQuery, RecipientQueryResponse},
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Mailbox {} => {
            let res = query_mailbox(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Route {
            denom,
            destination_domain,
        } => {
            let res = query_route(ctx, denom, destination_domain)?;
            res.to_json_value()
        },
        QueryMsg::Routes { start_after, limit } => {
            let res = query_routes(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Recipient(RecipientQuery::InterchainSecurityModule {}) => {
            let ism = query_interchain_security_module(ctx);
            let res = RecipientQueryResponse::InterchainSecurityModule(ism);
            res.to_json_value()
        },
        _ => todo!(),
    }
}

#[inline]
fn query_mailbox(ctx: ImmutableCtx) -> StdResult<Addr> {
    MAILBOX.load(ctx.storage)
}

#[inline]
fn query_route(ctx: ImmutableCtx, denom: Denom, destination_domain: Domain) -> StdResult<Route> {
    ROUTES.load(ctx.storage, (&denom, destination_domain))
}

#[inline]
fn query_routes(
    ctx: ImmutableCtx,
    start_after: Option<QueryRoutesPageParam>,
    limit: Option<u32>,
) -> StdResult<Vec<QueryRoutesResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|p| Bound::Exclusive((&p.denom, p.destination_domain)));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    ROUTES
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| {
            let ((denom, destination_domain), route) = res?;
            Ok(QueryRoutesResponseItem {
                denom,
                destination_domain,
                route,
            })
        })
        .collect()
}

#[inline]
fn query_interchain_security_module(_ctx: ImmutableCtx) -> Option<Addr> {
    // Currently we just use the default ISM.
    None
}
