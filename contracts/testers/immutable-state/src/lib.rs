use {
    grug::{
        from_borsh_slice, from_json_slice, grug_derive, grug_export, to_json_vec,
        unwrap_into_generic_result, Addr, Coins, Context, ExternalApi, ExternalQuerier,
        ExternalStorage, GenericResult, Json, MutableCtx, QuerierWrapper, Region, Response,
        StdResult,
    },
    serde::de::DeserializeOwned,
};

#[grug_derive(serde)]
pub struct InstantiateMsg;

#[grug_derive(serde)]
pub struct ExecuteMsg;

#[grug_derive(serde)]
pub struct QueryMsg;

#[grug_export]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[grug_export]
pub fn execute(ctx: MutableCtx, _msg: ExecuteMsg) -> StdResult<Response> {
    ctx.querier.query_wasm_smart(ctx.contract, &QueryMsg)?;
    Ok(Response::default())
}

#[cfg(target_arch = "wasm32")]
mod __wasm_export_query {
    #[no_mangle]
    extern "C" fn query(ptr0: usize, ptr1: usize) -> usize {
        {
            super::_do_query(&super::query, ptr0, ptr1)
        }
    }
}
pub fn query(ctx: MutableCtx, _msg: QueryMsg) -> StdResult<Json> {
    ctx.storage.write(b"key", b"value");
    Ok(Json::default())
}

fn _do_query<M, E>(
    query_fn: &dyn Fn(MutableCtx, M) -> Result<Json, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = __do_query(query_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn __do_query<M, E>(
    query_fn: &dyn Fn(MutableCtx, M) -> Result<Json, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Json>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));

    // Fake mutable context
    let mutalbe_ctx = MutableCtx {
        storage: &mut ExternalStorage,
        api: &ExternalApi,
        querier: QuerierWrapper::new(&ExternalQuerier),
        chain_id: ctx.chain_id,
        block: ctx.block,
        contract: ctx.contract,
        sender: Addr::mock(1),
        funds: Coins::default(),
    };

    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

    query_fn(mutalbe_ctx, msg).into()
}
