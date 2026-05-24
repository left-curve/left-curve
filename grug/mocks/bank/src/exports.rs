use {
    crate::{
        ExecuteMsg, InstantiateMsg, QueryMsg, burn, force_transfer, initialize, mint,
        query_balance, query_balances, query_holders, query_supplies, query_supply, transfer,
    },
    anyhow::bail,
    grug_types::{
        BankMsg, BankQuery, BankQueryResponse, ImmutableCtx, Json, JsonSerExt, MutableCtx,
        Response, StdResult, SudoCtx,
    },
};

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    initialize(ctx.storage, msg.initial_balances)
}

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn receive(_ctx: MutableCtx) -> anyhow::Result<Response> {
    bail!("go away");
}

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Mint { to, denom, amount } => mint(ctx, to, denom, amount),
        ExecuteMsg::Burn {
            from,
            denom,
            amount,
        } => burn(ctx, from, denom, amount),
        ExecuteMsg::ForceTransfer {
            from,
            to,
            denom,
            amount,
        } => force_transfer(ctx, from, to, denom, amount),
    }
}

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Holders {
            denom,
            start_after,
            limit,
        } => query_holders(ctx.storage, denom, start_after, limit)?.to_json_value(),
    }
}

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> StdResult<Response> {
    for (to, coins) in msg.transfers {
        transfer(ctx.storage, msg.from, to, &coins)?;
    }

    Ok(Response::new())
}

#[rustfmt::skip]
#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn bank_query(ctx: ImmutableCtx, msg: BankQuery) -> StdResult<BankQueryResponse> {
    match msg {
        BankQuery::Balance(req) => {
            query_balance(ctx.storage, req).map(BankQueryResponse::Balance)
        },
        BankQuery::Balances(req) => {
            query_balances(ctx.storage, req).map(BankQueryResponse::Balances)
        },
        BankQuery::Supply(req) => {
            query_supply(ctx.storage, req).map(BankQueryResponse::Supply)
        },
        BankQuery::Supplies(req) => {
            query_supplies(ctx.storage, req).map(BankQueryResponse::Supplies)
        },
    }
}
