#[cfg(not(feature = "library"))]
use grug_macros::grug_export;
use {
    crate::{
        burn, initialize, mint, query_balance, query_balances, query_holders, query_supplies,
        query_supply, transfer, ExecuteMsg, InstantiateMsg, QueryMsg,
    },
    anyhow::bail,
    grug_types::{
        to_json_value, BankMsg, BankQuery, BankQueryResponse, ImmutableCtx, Json, MutableCtx,
        Response, StdResult, SudoCtx,
    },
};

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    initialize(ctx.storage, msg.initial_balances)
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn receive(_ctx: MutableCtx) -> anyhow::Result<Response> {
    // We do not expect anyone to send any fund to this contract.
    // Throw an error to revert the transfer.
    bail!("go away");
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Mint { to, denom, amount } => mint(ctx, to, denom, amount),
        ExecuteMsg::Burn {
            from,
            denom,
            amount,
        } => burn(ctx, from, denom, amount),
    }
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Holders {
            denom,
            start_after,
            limit,
        } => to_json_value(&query_holders(ctx.storage, denom, start_after, limit)?),
    }
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> StdResult<Response> {
    transfer(ctx.storage, &msg.from, &msg.to, &msg.coins)
}

#[cfg_attr(not(feature = "library"), grug_export)]
#[rustfmt::skip]
pub fn bank_query(ctx: ImmutableCtx, msg: BankQuery) -> StdResult<BankQueryResponse> {
    match msg {
        BankQuery::Balance { address, denom } => {
            query_balance(ctx.storage, address, denom).map(BankQueryResponse::Balance)
        },
        BankQuery::Balances { address, start_after, limit } => {
            query_balances(ctx.storage, address, start_after, limit).map(BankQueryResponse::Balances)
        },
        BankQuery::Supply { denom } => {
            query_supply(ctx.storage, denom).map(BankQueryResponse::Supply)
        },
        BankQuery::Supplies { start_after, limit } => {
            query_supplies(ctx.storage, start_after, limit).map(BankQueryResponse::Supplies)
        },
    }
}
