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

// Need to define these manually because we can't use the `grug_export` macro in
// this workspace, due to a cyclic reference issue (see comments in `Cargo.toml`).
#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
mod __wasm_exports {
    #[no_mangle]
    extern "C" fn instantiate(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_wasm::do_instantiate(&super::instantiate, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn receive(ctx_ptr: usize) -> usize {
        grug_wasm::do_receive(&super::receive, ctx_ptr)
    }

    #[no_mangle]
    extern "C" fn execute(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_wasm::do_execute(&super::execute, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn query(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_wasm::do_query(&super::query, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn bank_execute(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_wasm::do_bank_execute(&super::bank_execute, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn bank_query(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_wasm::do_bank_query(&super::bank_query, ctx_ptr, msg_ptr)
    }
}

pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    initialize(ctx.storage, msg.initial_balances)
}

pub fn receive(_ctx: MutableCtx) -> anyhow::Result<Response> {
    // We do not expect anyone to send any fund to this contract.
    // Throw an error to revert the transfer.
    bail!("go away");
}

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

pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Holders {
            denom,
            start_after,
            limit,
        } => to_json_value(&query_holders(ctx.storage, denom, start_after, limit)?),
    }
}

pub fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> StdResult<Response> {
    transfer(ctx.storage, &msg.from, &msg.to, &msg.coins)
}

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
