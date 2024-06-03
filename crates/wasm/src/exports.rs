use {
    crate::{
        make_auth_ctx, make_immutable_ctx, make_mutable_ctx, make_sudo_ctx,
        unwrap_into_generic_result, AuthCtx, ExternalApi, ExternalQuerier, ExternalStorage,
        ImmutableCtx, MutableCtx, Region, SudoCtx,
    },
    grug_types::{
        from_borsh_slice, from_json_slice, to_json_vec, BankMsg, BankQuery, BankQueryResponse,
        Context, GenericResult, IbcClientUpdateMsg, IbcClientVerifyMsg, Json, Response,
        SubMsgResult, Tx,
    },
    serde::de::DeserializeOwned,
};

// ----------------------------------- alloc -----------------------------------

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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
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
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let mutable_ctx = make_mutable_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
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
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let mutable_ctx = make_mutable_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

    execute_fn(mutable_ctx, msg).into()
}

// ----------------------------------- query -----------------------------------

pub fn do_query<M, E>(
    query_fn: &dyn Fn(ImmutableCtx, M) -> Result<Json, E>,
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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_query<M, E>(
    query_fn: &dyn Fn(ImmutableCtx, M) -> Result<Json, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Json>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let immutable_ctx = make_immutable_ctx!(ctx, &ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
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
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let mutable_ctx = make_mutable_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

    migrate_fn(mutable_ctx, msg).into()
}

// ----------------------------------- reply -----------------------------------

pub fn do_reply<M, E>(
    reply_fn: &dyn Fn(SudoCtx, M, SubMsgResult) -> Result<Response, E>,
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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_reply<M, E>(
    reply_fn: &dyn Fn(SudoCtx, M, SubMsgResult) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
    events_bytes: &[u8],
) -> GenericResult<Response>
where
    M: DeserializeOwned,
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));
    let events = unwrap_into_generic_result!(from_json_slice(events_bytes));

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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_receive<E>(
    receive_fn: &dyn Fn(MutableCtx) -> Result<Response, E>,
    ctx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let mutable_ctx = make_mutable_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_before_block<E>(
    before_block_fn: &dyn Fn(SudoCtx) -> Result<Response, E>,
    ctx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_after_block<E>(
    after_block_fn: &dyn Fn(SudoCtx) -> Result<Response, E>,
    ctx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_before_tx<E>(
    before_tx_fn: &dyn Fn(AuthCtx, Tx) -> Result<Response, E>,
    ctx_bytes: &[u8],
    tx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let auth_ctx = make_auth_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let tx = unwrap_into_generic_result!(from_json_slice(tx_bytes));

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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_after_tx<E>(
    after_tx_fn: &dyn Fn(AuthCtx, Tx) -> Result<Response, E>,
    ctx_bytes: &[u8],
    tx_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let auth_ctx = make_auth_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let tx = unwrap_into_generic_result!(from_json_slice(tx_bytes));

    after_tx_fn(auth_ctx, tx).into()
}

// ------------------------------- bank transfer -------------------------------

pub fn do_bank_execute<E>(
    transfer_fn: &dyn Fn(SudoCtx, BankMsg) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_bank_execute(transfer_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_bank_execute<E>(
    transfer_fn: &dyn Fn(SudoCtx, BankMsg) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

    transfer_fn(sudo_ctx, msg).into()
}

// -------------------------------- bank query ---------------------------------

pub fn do_bank_query<E>(
    query_fn: &dyn Fn(ImmutableCtx, BankQuery) -> Result<BankQueryResponse, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_bank_query(query_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_bank_query<E>(
    query_fn: &dyn Fn(ImmutableCtx, BankQuery) -> Result<BankQueryResponse, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<BankQueryResponse>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let immutable_ctx = make_immutable_ctx!(ctx, &ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

    query_fn(immutable_ctx, msg).into()
}

// ----------------------------- ibc client create -----------------------------

pub fn do_ibc_client_create<E>(
    create_fn: &dyn Fn(SudoCtx, Json, Json) -> Result<Response, E>,
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
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_ibc_client_create<E>(
    create_fn: &dyn Fn(SudoCtx, Json, Json) -> Result<Response, E>,
    ctx_bytes: &[u8],
    client_state_bytes: &[u8],
    consensus_state_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let client_state_bytes = unwrap_into_generic_result!(from_json_slice(client_state_bytes));
    let consensus_state_bytes = unwrap_into_generic_result!(from_json_slice(consensus_state_bytes));

    create_fn(sudo_ctx, client_state_bytes, consensus_state_bytes).into()
}

// ----------------------------- ibc client update -----------------------------

pub fn do_ibc_client_update<E>(
    update_fn: &dyn Fn(SudoCtx, IbcClientUpdateMsg) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_ibc_client_update(update_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_ibc_client_update<E>(
    update_fn: &dyn Fn(SudoCtx, IbcClientUpdateMsg) -> Result<Response, E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<Response>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let sudo_ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

    update_fn(sudo_ctx, msg).into()
}

// ----------------------------- ibc client verify -----------------------------

pub fn do_ibc_client_verify<E>(
    verify_fn: &dyn Fn(ImmutableCtx, IbcClientVerifyMsg) -> Result<(), E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    E: ToString,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = _do_ibc_client_verify(verify_fn, &ctx_bytes, &msg_bytes);
    let res_bytes = to_json_vec(&res).unwrap();

    Region::release_buffer(res_bytes) as usize
}

fn _do_ibc_client_verify<E>(
    verify_fn: &dyn Fn(ImmutableCtx, IbcClientVerifyMsg) -> Result<(), E>,
    ctx_bytes: &[u8],
    msg_bytes: &[u8],
) -> GenericResult<()>
where
    E: ToString,
{
    let ctx: Context = unwrap_into_generic_result!(from_borsh_slice(ctx_bytes));
    let immutable_ctx = make_immutable_ctx!(ctx, &ExternalStorage, &ExternalApi, &ExternalQuerier);
    let msg = unwrap_into_generic_result!(from_json_slice(msg_bytes));

    verify_fn(immutable_ctx, msg).into()
}
