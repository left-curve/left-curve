use {
    crate::Host,
    cw_std::{from_json, to_json, Binary, ContractResult, Response},
    serde::ser::Serialize,
};

pub fn call_instantiate<T, M>(host: &mut Host<T>, msg: &M) -> anyhow::Result<ContractResult<Response>>
where
    M: Serialize,
{
    // serialize message and load it into Wasm memory
    let msg_bytes = to_json(msg)?;
    let msg_ptr = host.write_to_memory(msg_bytes.as_ref())?;

    // call the instantiate function
    // note, we use host.read_then_wipe to deallocate the res_ptr
    let res_ptr: u32 = host.call("instantiate", msg_ptr)?;
    let res_bytes = host.read_then_wipe(res_ptr)?;

    from_json(res_bytes)

    // no need to deallocate msg_ptr, they were already freed in Wasm code.
    // the send function doesn't have response data either, so we're done.
}

pub fn call_execute<T, M>(host: &mut Host<T>, msg: &M) -> anyhow::Result<ContractResult<Response>>
where
    M: Serialize,
{
    let msg_bytes = to_json(msg)?;
    let msg_ptr = host.write_to_memory(msg_bytes.as_ref())?;

    let res_ptr: u32 = host.call("execute", msg_ptr)?;
    let res_bytes = host.read_then_wipe(res_ptr)?;

    from_json(res_bytes)
}

pub fn call_query<T, M>(host: &mut Host<T>, msg: &M) -> anyhow::Result<ContractResult<Binary>>
where
    M: Serialize,
{
    let msg_bytes = to_json(msg)?;
    let msg_ptr = host.write_to_memory(msg_bytes.as_ref())?;

    let res_ptr: u32 = host.call("query", msg_ptr)?;
    let res_bytes = host.read_then_wipe(res_ptr)?;

    from_json(res_bytes)
}
