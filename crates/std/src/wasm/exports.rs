use {
    crate::{
        from_json, to_json, AuthCtx, BankQuery, BankQueryResponse, Binary, Context, Event,
        ExternalStorage, GenericResult, IbcClientExecuteMsg, IbcClientQueryMsg,
        IbcClientQueryResponse, ImmutableCtx, MutableCtx, Region, Response, StdError, SudoCtx,
        TransferMsg, Tx,
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

macro_rules! try_unwrap_field {
    ($field:expr, $name:literal) => {
        match $field {
            Some(field) => field,
            None => {
                return Err(StdError::missing_context($name)).into();
            },
        }
    }
}

macro_rules! make_immutable_ctx {
    ($ctx:ident) => {
        ImmutableCtx {
            store:           &ExternalStorage,
            chain_id:        $ctx.chain_id,
            block_height:    $ctx.block_height,
            block_timestamp: $ctx.block_timestamp,
            block_hash:      $ctx.block_hash,
            contract:        $ctx.contract,
        }
    }
}

macro_rules! make_mutable_ctx {
    ($ctx:ident) => {
        MutableCtx {
            store:           &mut ExternalStorage,
            chain_id:        $ctx.chain_id,
            block_height:    $ctx.block_height,
            block_timestamp: $ctx.block_timestamp,
            block_hash:      $ctx.block_hash,
            contract:        $ctx.contract,
            sender:          try_unwrap_field!($ctx.sender, "sender"),
            funds:           try_unwrap_field!($ctx.funds, "funds"),
        }
    }
}

macro_rules! make_sudo_ctx {
    ($ctx:ident) => {
        SudoCtx {
            store:           &mut ExternalStorage,
            chain_id:        $ctx.chain_id,
            block_height:    $ctx.block_height,
            block_timestamp: $ctx.block_timestamp,
            block_hash:      $ctx.block_hash,
            contract:        $ctx.contract,
        }
    }
}

macro_rules! make_auth_ctx {
    ($ctx:ident) => {
        AuthCtx {
            store:           &mut ExternalStorage,
            chain_id:        $ctx.chain_id,
            block_height:    $ctx.block_height,
            block_timestamp: $ctx.block_timestamp,
            block_hash:      $ctx.block_hash,
            contract:        $ctx.contract,
            simulate:        try_unwrap_field!($ctx.simulate, "simulate"),
        }
    }
}

// -------------------------------- instantiate --------------------------------

pub fn do_instantiate<M, E>(
    instantiate_fn: &dyn Fn(MutableCtx, M) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
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
    instantiate_fn: &dyn Fn(MutableCtx, M) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Response>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let mutable_ctx = make_mutable_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));

    instantiate_fn(mutable_ctx, msg).into()
}

// ---------------------------------- execute ----------------------------------

pub fn do_execute<M, E>(
    execute_fn: &dyn Fn(MutableCtx, M) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
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
    execute_fn: &dyn Fn(MutableCtx, M) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Response>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let mutable_ctx = make_mutable_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));

    execute_fn(mutable_ctx, msg).into()
}

// ----------------------------------- query -----------------------------------

pub fn do_query<M, E>(
    query_fn: &dyn Fn(ImmutableCtx, M) -> Result<Binary, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
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
    query_fn: &dyn Fn(ImmutableCtx, M) -> Result<Binary, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Binary>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let immutable_ctx = make_immutable_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));

    query_fn(immutable_ctx, msg).into()
}

// ---------------------------------- migrate ----------------------------------

pub fn do_migrate<M, E>(
    migrate_fn: &dyn Fn(MutableCtx, M) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
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
    migrate_fn: &dyn Fn(MutableCtx, M) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Response>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let mutable_ctx = make_mutable_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));

    migrate_fn(mutable_ctx, msg).into()
}

// ----------------------------------- reply -----------------------------------

pub fn do_reply<M, E>(
    reply_fn: &dyn Fn(SudoCtx, M, GenericResult<Vec<Event>>) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
    events_ptr: usize,
) -> usize
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };
    let events_bytes = unsafe { Region::consume(events_ptr as *mut Region) };

    let res = _do_reply(reply_fn, &ctx_bytes, &msg_bytes, &events_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_reply<M, E>(
    reply_fn: &dyn Fn(SudoCtx, M, GenericResult<Vec<Event>>) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
    events_bytes: &[u8],
) -> GenericResult<Response>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));
    let events = try_into_generic_result!(from_json(events_bytes));

    reply_fn(sudo_ctx, msg, events).into()
}

// ---------------------------------- receive ----------------------------------

pub fn do_receive<E>(
    receive_fn: &dyn Fn(MutableCtx) -> Result<Response, E>,
    ctx_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };

    let res = _do_receive(receive_fn, &ctx_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_receive<E>(
    receive_fn: &dyn Fn(MutableCtx) -> Result<Response, E>,
    ctx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let mutable_ctx = make_mutable_ctx!(ctx);

    receive_fn(mutable_ctx).into()
}

// ------------------------------- before block --------------------------------

pub fn do_before_block<E>(
    before_block_fn: &dyn Fn(SudoCtx) -> Result<Response, E>,
    ctx_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };

    let res = _do_before_block(before_block_fn, &ctx_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_before_block<E>(
    before_block_fn: &dyn Fn(SudoCtx) -> Result<Response, E>,
    ctx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx);

    before_block_fn(sudo_ctx).into()
}

// -------------------------------- after block --------------------------------

pub fn do_after_block<E>(
    after_block_fn: &dyn Fn(SudoCtx) -> Result<Response, E>,
    ctx_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };

    let res = _do_after_block(after_block_fn, &ctx_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_after_block<E>(
    after_block_fn: &dyn Fn(SudoCtx) -> Result<Response, E>,
    ctx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx);

    after_block_fn(sudo_ctx).into()
}

// --------------------------------- before tx ---------------------------------

pub fn do_before_tx<E>(
    before_tx_fn: &dyn Fn(AuthCtx, Tx) -> Result<Response, E>,
    ctx_ptr: usize,
    tx_ptr: usize,
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
    before_tx_fn: &dyn Fn(AuthCtx, Tx) -> Result<Response, E>,
    ctx_bytes: &[u8],
    tx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let auth_ctx = make_auth_ctx!(ctx);
    let tx = try_into_generic_result!(from_json(tx_bytes));

    before_tx_fn(auth_ctx, tx).into()
}

// --------------------------------- after tx ----------------------------------

pub fn do_after_tx<E>(
    after_tx_fn: &dyn Fn(AuthCtx, Tx) -> Result<Response, E>,
    ctx_ptr: usize,
    tx_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let tx_bytes = unsafe { Region::consume(tx_ptr as *mut Region) };

    let res = _do_after_tx(after_tx_fn, &ctx_bytes, &tx_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_after_tx<E>(
    after_tx_fn: &dyn Fn(AuthCtx, Tx) -> Result<Response, E>,
    ctx_bytes: &[u8],
    tx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let auth_ctx = make_auth_ctx!(ctx);
    let tx = try_into_generic_result!(from_json(tx_bytes));

    after_tx_fn(auth_ctx, tx).into()
}

// --------------------------------- transfer ----------------------------------

pub fn do_transfer<E>(
    transfer_fn: &dyn Fn(SudoCtx, TransferMsg) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
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
    transfer_fn: &dyn Fn(SudoCtx, TransferMsg) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));

    transfer_fn(sudo_ctx, msg).into()
}

// -------------------------------- bank query ---------------------------------

pub fn do_query_bank<E>(
    query_bank_fn: &dyn Fn(ImmutableCtx, BankQuery) -> Result<BankQueryResponse, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
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
    query_bank_fn: &dyn Fn(ImmutableCtx, BankQuery) -> Result<BankQueryResponse, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<BankQueryResponse>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let immutable_ctx = make_immutable_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));

    query_bank_fn(immutable_ctx, msg).into()
}

// ----------------------------- ibc client create -----------------------------

pub fn do_ibc_client_create<E>(
    create_fn: &dyn Fn(SudoCtx, Binary, Binary) -> Result<Response, E>,
    ctx_ptr: usize,
    client_state_ptr: usize,
    consensus_state_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let client_state_bytes = unsafe { Region::consume(client_state_ptr as *mut Region) };
    let consensus_state_bytes = unsafe { Region::consume(consensus_state_ptr as *mut Region) };

    let res = _do_ibc_client_create(
        create_fn,
        &ctx_bytes,
        &client_state_bytes,
        &consensus_state_bytes,
    );
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_ibc_client_create<E>(
    create_fn: &dyn Fn(SudoCtx, Binary, Binary) -> Result<Response, E>,
    ctx_bytes: &[u8],
    client_state_bytes: &[u8],
    consensus_state_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx);
    let client_state_bytes = try_into_generic_result!(from_json(client_state_bytes));
    let consensus_state_bytes = try_into_generic_result!(from_json(consensus_state_bytes));

    create_fn(sudo_ctx, client_state_bytes, consensus_state_bytes).into()
}

// ---------------------------- ibc client execute -----------------------------

pub fn do_ibc_client_execute<E>(
    execute_fn: &dyn Fn(SudoCtx, IbcClientExecuteMsg) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_ibc_client_execute(execute_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_ibc_client_execute<E>(
    execute_fn: &dyn Fn(SudoCtx, IbcClientExecuteMsg) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));

    execute_fn(sudo_ctx, msg).into()
}

// ----------------------------- ibc client query ------------------------------

pub fn do_ibc_client_query<E>(
    query_fn: &dyn Fn(ImmutableCtx, IbcClientQueryMsg) -> Result<IbcClientQueryResponse, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_ibc_client_query(query_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json(&res).unwrap();

    Region::release_buffer(res_bytes.into()) as usize
}

fn _do_ibc_client_query<E>(
    query_fn: &dyn Fn(ImmutableCtx, IbcClientQueryMsg) -> Result<IbcClientQueryResponse, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<IbcClientQueryResponse>
where
    E: ToString,
{
    let ctx: Context = try_into_generic_result!(from_json(ctx_bytes));
    let immutable_ctx = make_immutable_ctx!(ctx);
    let msg = try_into_generic_result!(from_json(msg_bytes));

    query_fn(immutable_ctx, msg).into()
}
