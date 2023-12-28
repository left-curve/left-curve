use {
    crate::{ContractResult, ExecuteCtx, ExternalStorage, Region, Response},
    serde::de::DeserializeOwned,
};

// TODO: replace with https://doc.rust-lang.org/std/ops/trait.Try.html once stabilized
macro_rules! try_into_contract_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                return ContractResult::Err(err.to_string());
            },
        }
    };
}

pub fn do_execute<M, E>(
    execute_fn: &dyn Fn(ExecuteCtx, M) -> Result<Response, E>,
    msg_ptr:    usize,
) -> usize
where
    M: DeserializeOwned,
    E: ToString,
{
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_execute(execute_fn, &msg_bytes);
    let res_bytes = serde_json_wasm::to_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_execute<M, E>(
    execute_fn: &dyn Fn(ExecuteCtx, M) -> Result<Response, E>,
    msg_bytes:  &[u8],
) -> ContractResult
where
    M: DeserializeOwned,
    E: ToString,
{
    let msg = try_into_contract_result!(serde_json_wasm::from_slice(msg_bytes));

    let ctx = ExecuteCtx {
        store: &mut ExternalStorage,
        // TODO: other fields...
    };

    execute_fn(ctx, msg).into()
}
