use {
    crate::{
        burn, initialize, mint, query_balance, query_balances, query_supplies, query_supply,
        transfer, ExecuteMsg, InstantiateMsg,
    },
    anyhow::bail,
    grug::{
        grug_export, BankMsg, BankQuery, BankQueryResponse, ImmutableCtx, MutableCtx, Response,
        StdResult, SudoCtx,
    },
};

#[grug_export]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    initialize(ctx.storage, msg.initial_balances)
}

#[grug_export]
pub fn receive(_ctx: MutableCtx) -> anyhow::Result<Response> {
    // We do not expect anyone to send any fund to this contract.
    // Throw an error to revert the transfer.
    bail!("Go away");
}

#[grug_export]
pub fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> StdResult<Response> {
    transfer(ctx.storage, &msg.from, &msg.to, &msg.coins)
}

#[grug_export]
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

#[grug_export]
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
