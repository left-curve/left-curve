use grug::{grug_export, Empty, MutableCtx, Region, Response, StdResult};

#[grug_export]
pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[grug_export]
pub fn execute(ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
    // Call the contract's own `query` function.
    ctx.querier.query_wasm_smart(ctx.contract, &Empty {})?;

    Ok(Response::new())
}

extern "C" {
    fn db_write(key_ptr: usize, value_ptr: usize);
}

#[no_mangle]
extern "C" fn query(_ctx_ptr: usize, _msg_ptr: usize) -> usize {
    let key = b"larry";
    let key_region = Region::build(key);
    let key_ptr = &*key_region as *const Region;

    let value = b"engineer";
    let value_region = Region::build(value);
    let value_ptr = &*value_region as *const Region;

    // This should fail!
    unsafe {
        db_write(key_ptr as usize, value_ptr as usize);
    }

    0
}
