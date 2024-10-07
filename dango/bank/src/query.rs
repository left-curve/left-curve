use {
    crate::{BALANCES, NAMESPACE_OWNERS, SUPPLIES},
    dango_types::bank::QueryMsg,
    grug::{
        Addr, BankQuery, BankQueryResponse, Bound, Coin, Coins, Denom, ImmutableCtx, Json,
        JsonSerExt, NumberConst, Order, Part, StdResult, Uint128,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Namespace { namespace } => {
            let res = query_namespace(ctx, namespace)?;
            res.to_json_value()
        },
        QueryMsg::Namespaces { start_after, limit } => {
            let res = query_namespaces(ctx, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_namespace(ctx: ImmutableCtx, namespace: Part) -> StdResult<Addr> {
    NAMESPACE_OWNERS.load(ctx.storage, &namespace)
}

fn query_namespaces(
    ctx: ImmutableCtx,
    start_after: Option<Part>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Part, Addr>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    NAMESPACE_OWNERS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn bank_query(ctx: ImmutableCtx, msg: BankQuery) -> StdResult<BankQueryResponse> {
    match msg {
        BankQuery::Balance { address, denom } => {
            let res = query_balance(ctx, address, denom)?;
            Ok(BankQueryResponse::Balance(res))
        },
        BankQuery::Balances {
            address,
            start_after,
            limit,
        } => {
            let res = query_balances(ctx, address, start_after, limit)?;
            Ok(BankQueryResponse::Balances(res))
        },
        BankQuery::Supply { denom } => {
            let res = query_supply(ctx, denom)?;
            Ok(BankQueryResponse::Supply(res))
        },
        BankQuery::Supplies { start_after, limit } => {
            let res = query_supplies(ctx, start_after, limit)?;
            Ok(BankQueryResponse::Supplies(res))
        },
    }
}

fn query_balance(ctx: ImmutableCtx, address: Addr, denom: Denom) -> StdResult<Coin> {
    let maybe_amount = BALANCES.may_load(ctx.storage, (&address, &denom))?;

    Ok(Coin {
        denom,
        amount: maybe_amount.unwrap_or(Uint128::ZERO),
    })
}

fn query_balances(
    ctx: ImmutableCtx,
    address: Addr,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<Coins> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    BALANCES
        .prefix(&address)
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<BTreeMap<_, _>>>()?
        .try_into()
}

fn query_supply(ctx: ImmutableCtx, denom: Denom) -> StdResult<Coin> {
    let maybe_supply = SUPPLIES.may_load(ctx.storage, &denom)?;

    Ok(Coin {
        denom,
        amount: maybe_supply.unwrap_or(Uint128::ZERO),
    })
}

fn query_supplies(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<Coins> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    SUPPLIES
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<BTreeMap<_, _>>>()?
        .try_into()
}
