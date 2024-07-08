use {
    crate::execute,
    grug::{
        from_borsh_slice, make_mutable_ctx, to_json_vec, unwrap_into_generic_result, Context,
        ExternalApi, ExternalQuerier, ExternalStorage, GenericResult, MutableCtx, QuerierWrapper,
        Region, Response,
    },
};

#[no_mangle]
extern "C" fn execute_borsh(ctx_ptr: usize, msg_ptr: usize) -> usize {
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = || -> GenericResult<Response> {
        let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
        let mutable_ctx =
            make_mutable_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
        let msg = unwrap_into_generic_result!(from_borsh_slice(msg_bytes));
        execute(mutable_ctx, msg).into()
    }();

    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}
