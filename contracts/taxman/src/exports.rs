use {
    crate::{query_config, update_config, ExecuteMsg, QueryMsg},
    grug_types::{to_json_value, ImmutableCtx, Json, MutableCtx, Response, StdResult},
};

// Need to define these manually because we can't use the `grug::export` macro in
// this workspace, due to a cyclic reference issue (see comments in `Cargo.toml`).
#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
mod __wasm_exports {
    #[no_mangle]
    extern "C" fn instantiate(ctx_ptr: usize, msg_ptr: usize) -> usize {
        grug_ffi::do_instantiate(&crate::initialize, ctx_ptr, msg_ptr)
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
    extern "C" fn handle_fee(ctx_ptr: usize, tx_ptr: usize, outcome_ptr: usize) -> usize {
        grug_ffi::do_handle_fee(
            &crate::compute_and_transfer_fee,
            ctx_ptr,
            tx_ptr,
            outcome_ptr,
        )
    }
}

pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { new_cfg } => update_config(ctx, &new_cfg),
    }
}

pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Config {} => to_json_value(&query_config(ctx.storage)?),
    }
}
