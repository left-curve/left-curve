use {
    crate::{ExecuteMsg, QueryMsg, query_state, update_key},
    grug_types::{ImmutableCtx, Json, JsonSerExt, MutableCtx, Response, StdResult},
};

// Need to define these manually because we can't use the `grug::export` macro in
// this workspace, due to a cyclic reference issue (see comments in `Cargo.toml`).
#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
mod __wasm_exports {
    #[no_mangle]
    extern "C" fn instantiate(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_ffi::do_instantiate(&crate::instantiate, ctx_ptr, msg_ptr)
    }

    #[no_mangle]
    extern "C" fn authenticate(ctx_ptr: usize, tx_ptr: usize) -> usize {
        grug_ffi::do_authenticate(&crate::authenticate, ctx_ptr, tx_ptr)
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
        QueryMsg::State {} => query_state(ctx.storage)?.to_json_value(),
    }
}
