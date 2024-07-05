use {
    crate::{
        authenticate_tx, initialize, query_state, update_key, ExecuteMsg, InstantiateMsg, QueryMsg,
    },
    grug_types::{to_json_value, AuthCtx, ImmutableCtx, Json, MutableCtx, Response, StdResult, Tx},
};

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
mod __wasm_exports {
    #[no_mangle]
    extern "C" fn instantiate(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_wasm::do_instantiate(&super::instantiate, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn before_tx(ctx_ptr: usize, tx_ptr: usize) -> usize {
        grug_wasm::do_before_tx(&super::before_tx, ctx_ptr, tx_ptr)
    }

    #[no_mangle]
    extern "C" fn after_tx(ctx_ptr: usize, tx_ptr: usize) -> usize {
        grug_wasm::do_after_tx(&super::after_tx, ctx_ptr, tx_ptr)
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
}

pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    initialize(ctx.storage, &msg.public_key)
}

pub fn before_tx(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
    authenticate_tx(ctx, tx)
}

pub fn after_tx(_ctx: AuthCtx, _tx: Tx) -> StdResult<Response> {
    // Nothing to do
    Ok(Response::new())
}

pub fn receive(_ctx: MutableCtx) -> StdResult<Response> {
    // Do nothing, accept all transfers.
    Ok(Response::new())
}

pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateKey { new_public_key } => update_key(ctx, &new_public_key),
    }
}

pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::State {} => to_json_value(&query_state(ctx.storage)?),
    }
}
