use {
    crate::{
        from_json, to_json, BankQuery, BankQueryResponse, BeforeTxCtx, Binary, Context, ExecuteCtx,
        ExternalStorage, GenericResult, InstantiateCtx, MigrateCtx, QueryCtx, Region, Response,
        TransferCtx, TransferMsg, Tx,
    },
    serde::de::DeserializeOwned,
};

/// Reserve a region in Wasm memory of the given number of bytes. Return the
/// memory address of a Region object that describes the memory region that was
/// reserved.
///
/// This is used by the host to pass non-primitive data into the Wasm module.
#[no_mangle]
extern "C" fn allocate(capacity: usize) -> usize {
    let data = Vec::<u8>::with_capacity(capacity);
    Region::release_buffer(data) as usize
}

/// Free the specified region in the Wasm module's linear memory.
#[no_mangle]
extern "C" fn deallocate(region_addr: usize) {
    let _ = unsafe { Region::consume(region_addr as *mut Region) };
    // data is dropped here, which calls Vec<u8> destructor, freeing the memory
}

// TODO: replace with https://doc.rust-lang.org/std/ops/trait.Try.html once stabilized
macro_rules! try_into_generic_result {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                return GenericResult::Err(err.to_string());
            },
        }
    };
}

// -------------------------------- instantiate --------------------------------

pub fn do_instantiate<M, E>(
    instantiate_fn: &dyn Fn(InstantiateCtx, M) -> Result<Response, E>,
    ctx_ptr:        usize,
    msg_ptr:        usize,
) -> usize
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_instantiate(instantiate_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_instantiate<M, E>(
    instantiate_fn: &dyn Fn(InstantiateCtx, M) -> Result<Response, E>,
    ctx_bytes:      &[u8],
    msg_bytes:      &[u8],
) -> GenericResult<Response>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let msg = try_into_generic_result!(from_json(msg_bytes));

    let ctx = InstantiateCtx {
        store:    &mut ExternalStorage,
        block:    ctx.block,
        contract: ctx.contract,
        sender:   ctx.sender.expect("host failed to provide a sender"),
        funds:    ctx.funds.expect("host failed to provide funds"),
    };

    instantiate_fn(ctx, msg).into()
}

// ---------------------------------- execute ----------------------------------

pub fn do_execute<M, E>(
    execute_fn: &dyn Fn(ExecuteCtx, M) -> Result<Response, E>,
    ctx_ptr:    usize,
    msg_ptr:    usize,
) -> usize
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_execute(execute_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_execute<M, E>(
    execute_fn: &dyn Fn(ExecuteCtx, M) -> Result<Response, E>,
    ctx_bytes:  &[u8],
    msg_bytes:  &[u8],
) -> GenericResult<Response>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let msg = try_into_generic_result!(from_json(msg_bytes));

    let ctx = ExecuteCtx {
        store:    &mut ExternalStorage,
        block:    ctx.block,
        contract: ctx.contract,
        sender:   ctx.sender.expect("host failed to provide a sender"),
        funds:    ctx.funds.expect("host failed to provide funds"),
    };

    execute_fn(ctx, msg).into()
}

// ----------------------------------- query -----------------------------------

pub fn do_query<M, E>(
    query_fn: &dyn Fn(QueryCtx, M) -> Result<Binary, E>,
    ctx_ptr:  usize,
    msg_ptr:  usize,
) -> usize
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_query(query_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_query<M, E>(
    query_fn:  &dyn Fn(QueryCtx, M) -> Result<Binary, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Binary>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let msg = try_into_generic_result!(from_json(msg_bytes));

    let ctx = QueryCtx {
        store:    &ExternalStorage,
        block:    ctx.block,
        contract: ctx.contract,
    };

    query_fn(ctx, msg).into()
}

// ---------------------------------- migrate ----------------------------------

pub fn do_migrate<M, E>(
    migrate_fn: &dyn Fn(MigrateCtx, M) -> Result<Response, E>,
    ctx_ptr:    usize,
    msg_ptr:    usize,
) -> usize
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_migrate(migrate_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_migrate<M, E>(
    migrate_fn: &dyn Fn(MigrateCtx, M) -> Result<Response, E>,
    ctx_bytes:  &[u8],
    msg_bytes:  &[u8],
) -> GenericResult<Response>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let msg = try_into_generic_result!(from_json(msg_bytes));

    let ctx = MigrateCtx {
        store:    &mut ExternalStorage,
        block:    ctx.block,
        contract: ctx.contract,
        sender:   ctx.sender.expect("host failed to provide a sender"),
    };

    migrate_fn(ctx, msg).into()
}

// --------------------------------- before tx ---------------------------------

pub fn do_before_tx<E>(
    before_tx_fn: &dyn Fn(BeforeTxCtx, Tx) -> Result<Response, E>,
    ctx_ptr:      usize,
    tx_ptr:       usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let tx_bytes = unsafe { Region::consume(tx_ptr as *mut Region) };

    let res = _do_before_tx(before_tx_fn, &ctx_bytes, &tx_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_before_tx<E>(
    before_tx_fn: &dyn Fn(BeforeTxCtx, Tx) -> Result<Response, E>,
    ctx_bytes:    &[u8],
    tx_bytes:     &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let tx = try_into_generic_result!(from_json(tx_bytes));

    let ctx = BeforeTxCtx {
        store:    &mut ExternalStorage,
        block:    ctx.block,
        contract: ctx.contract,
        simulate: ctx.simulate.expect("host failed to specify whether it's simulation mode"),
    };

    before_tx_fn(ctx, tx).into()
}

// --------------------------------- transfer ----------------------------------

pub fn do_transfer<E>(
    transfer_fn: &dyn Fn(TransferCtx, TransferMsg) -> Result<Response, E>,
    ctx_ptr:     usize,
    msg_ptr:     usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_transfer(transfer_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_transfer<E>(
    transfer_fn:  &dyn Fn(TransferCtx, TransferMsg) -> Result<Response, E>,
    ctx_bytes:    &[u8],
    msg_bytes:    &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let msg = try_into_generic_result!(from_json(msg_bytes));

    let ctx = TransferCtx {
        store:    &mut ExternalStorage,
        block:    ctx.block,
        contract: ctx.contract,
    };

    transfer_fn(ctx, msg).into()
}

// -------------------------------- bank query ---------------------------------

pub fn do_query_bank<E>(
    query_bank_fn: &dyn Fn(QueryCtx, BankQuery) -> Result<BankQueryResponse, E>,
    ctx_ptr:       usize,
    msg_ptr:       usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_query_bank(query_bank_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_query_bank<E>(
    query_bank_fn:  &dyn Fn(QueryCtx, BankQuery) -> Result<BankQueryResponse, E>,
    ctx_bytes:      &[u8],
    msg_bytes:      &[u8],
) -> GenericResult<BankQueryResponse>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let msg = try_into_generic_result!(from_json(msg_bytes));

    let ctx = QueryCtx {
        store:    &ExternalStorage,
        block:    ctx.block,
        contract: ctx.contract,
    };

    query_bank_fn(ctx, msg).into()
}
