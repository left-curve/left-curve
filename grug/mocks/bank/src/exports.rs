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

// Need to define these manually because we can't use the `grug::export` macro in
// this workspace, due to a cyclic reference issue (see comments in `Cargo.toml`).
#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
mod __wasm_exports {
    #[no_mangle]
    extern "C" fn instantiate(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_ffi::do_instantiate(&super::instantiate, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn receive(ctx_ptr: usize) -> usize {
        grug_ffi::do_receive(&super::receive, ctx_ptr)
    }

    #[no_mangle]
    extern "C" fn execute(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_ffi::do_execute(&super::execute, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn query(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_ffi::do_query(&super::query, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn bank_execute(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_ffi::do_bank_execute(&super::bank_execute, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn bank_query(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_ffi::do_bank_query(&super::bank_query, ctx_ptr, msg_ptr)
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

pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Holders {
            denom,
            start_after,
            limit,
        } => query_holders(ctx.storage, denom, start_after, limit)?.to_json_value(),
    }
}

pub fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> StdResult<Response> {
    transfer(ctx.storage, msg.from, msg.to, &msg.coins)
}

#[rustfmt::skip]
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
