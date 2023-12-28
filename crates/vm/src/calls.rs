use {
    crate::Host,
    cw_std::ContractResult,
    serde::ser::Serialize,
};

pub fn call_execute<T, M>(host: &mut Host<T>, msg: M) -> anyhow::Result<ContractResult>
where
    M: Serialize,
{
    // serialize message and load it into Wasm memory
    let msg_bytes = serde_json_wasm::to_vec(&msg)?;
    let msg_ptr = host.write_to_memory(&msg_bytes)?;

    // call the execute function
    // note, we use host.read_then_wipe to deallocate the res_ptr
    let res_ptr: u32 = host.call("execute", msg_ptr)?;
    let res_bytes = host.read_then_wipe(res_ptr)?;
    let res: ContractResult = serde_json_wasm::from_slice(&res_bytes)?;

    Ok(res)

    // no need to deallocate msg_ptr, they were already freed in Wasm code.
    // the send function doesn't have response data either, so we're done.
}
