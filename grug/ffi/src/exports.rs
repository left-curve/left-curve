use {
    crate::{ExternalApi, ExternalQuerier, ExternalStorage, Region},
    grug_types::{
        make_auth_ctx, make_immutable_ctx, make_mutable_ctx, make_sudo_ctx,
        unwrap_into_generic_result, AuthCtx, AuthResponse, BankMsg, BankQuery, BankQueryResponse,
        BorshDeExt, BorshSerExt, Context, GenericResult, GenericResultExt, ImmutableCtx, Json,
        JsonDeExt, MutableCtx, QuerierWrapper, Response, SubMsgResult, SudoCtx, Tx, TxOutcome,
    },
    serde::de::DeserializeOwned,
    std::fmt::Display,
};

/// Reserve a region in Wasm memory of the given number of bytes. Return the
/// memory address of a Region object that describes the memory region that was
/// reserved.
///
/// This is used by the host to pass non-primitive data into the Wasm module.
#[unsafe(no_mangle)]
extern "C" fn allocate(capacity: usize) -> usize {
    let data = Vec::<u8>::with_capacity(capacity);
    Region::release_buffer(data) as usize
}

/// Free the specified region in the Wasm module's linear memory.
#[unsafe(no_mangle)]
extern "C" fn deallocate(region_addr: usize) {
    let _ = unsafe { Region::consume(region_addr as *mut Region) };
    // data is dropped here, which calls Vec<u8> destructor, freeing the memory
}

pub fn do_instantiate<M, E>(
    instantiate_fn: &dyn Fn(MutableCtx, M) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    M: DeserializeOwned,
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_mutable_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

        let msg = unwrap_into_generic_result!(msg_bytes.deserialize_borsh::<Json>());
        let msg = unwrap_into_generic_result!(msg.deserialize_json());

        instantiate_fn(ctx, msg).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_execute<M, E>(
    execute_fn: &dyn Fn(MutableCtx, M) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    M: DeserializeOwned,
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_mutable_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

        let msg = unwrap_into_generic_result!(msg_bytes.deserialize_borsh::<Json>());
        let msg = unwrap_into_generic_result!(msg.deserialize_json());

        execute_fn(ctx, msg).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_query<M, E>(
    query_fn: &dyn Fn(ImmutableCtx, M) -> Result<Json, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    M: DeserializeOwned,
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let immutable_ctx =
            make_immutable_ctx!(ctx, &ExternalStorage, &ExternalApi, &ExternalQuerier);

        let msg = unwrap_into_generic_result!(msg_bytes.deserialize_borsh::<Json>());
        let msg = unwrap_into_generic_result!(msg.deserialize_json());

        query_fn(immutable_ctx, msg).into_generic_result()
    })();
    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_migrate<M, E>(
    migrate_fn: &dyn Fn(SudoCtx, M) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    M: DeserializeOwned,
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

        let msg = unwrap_into_generic_result!(msg_bytes.deserialize_borsh::<Json>());
        let msg = unwrap_into_generic_result!(msg.deserialize_json());

        migrate_fn(ctx, msg).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_reply<M, E>(
    reply_fn: &dyn Fn(SudoCtx, M, SubMsgResult) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
    events_ptr: usize,
) -> usize
where
    M: DeserializeOwned,
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };
    let events_bytes = unsafe { Region::consume(events_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

        let msg = unwrap_into_generic_result!(msg_bytes.deserialize_borsh::<Json>());
        let msg = unwrap_into_generic_result!(msg.deserialize_json());

        let events = unwrap_into_generic_result!(events_bytes.deserialize_borsh());

        reply_fn(ctx, msg, events).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_receive<E>(
    receive_fn: &dyn Fn(MutableCtx) -> Result<Response, E>,
    ctx_ptr: usize,
) -> usize
where
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_mutable_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

        receive_fn(ctx).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_cron_execute<E>(
    cron_execute_fn: &dyn Fn(SudoCtx) -> Result<Response, E>,
    ctx_ptr: usize,
) -> usize
where
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);

        cron_execute_fn(ctx).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_authenticate<E>(
    authenticate_fn: &dyn Fn(AuthCtx, Tx) -> Result<AuthResponse, E>,
    ctx_ptr: usize,
    tx_ptr: usize,
) -> usize
where
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let tx_bytes = unsafe { Region::consume(tx_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_auth_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
        let tx = unwrap_into_generic_result!(tx_bytes.deserialize_borsh());

        authenticate_fn(ctx, tx).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_backrun<E>(
    backrun_fn: &dyn Fn(AuthCtx, Tx) -> Result<Response, E>,
    ctx_ptr: usize,
    tx_ptr: usize,
) -> usize
where
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let tx_bytes = unsafe { Region::consume(tx_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_auth_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
        let tx = unwrap_into_generic_result!(tx_bytes.deserialize_borsh());

        backrun_fn(ctx, tx).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_bank_execute<E>(
    transfer_fn: &dyn Fn(SudoCtx, BankMsg) -> Result<Response, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_sudo_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
        let msg = unwrap_into_generic_result!(msg_bytes.deserialize_borsh());

        transfer_fn(ctx, msg).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_bank_query<E>(
    query_fn: &dyn Fn(ImmutableCtx, BankQuery) -> Result<BankQueryResponse, E>,
    ctx_ptr: usize,
    msg_ptr: usize,
) -> usize
where
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let msg_bytes = unsafe { Region::consume(msg_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let ctx = make_immutable_ctx!(ctx, &ExternalStorage, &ExternalApi, &ExternalQuerier);
        let msg = unwrap_into_generic_result!(msg_bytes.deserialize_borsh());

        query_fn(ctx, msg).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_withhold_fee<E>(
    withhold_fee_fn: &dyn Fn(AuthCtx, Tx) -> Result<Response, E>,
    ctx_ptr: usize,
    tx_ptr: usize,
) -> usize
where
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let tx_bytes = unsafe { Region::consume(tx_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let auth_ctx = make_auth_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
        let tx = unwrap_into_generic_result!(tx_bytes.deserialize_borsh());

        withhold_fee_fn(auth_ctx, tx).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}

pub fn do_finalize_fee<E>(
    finalize_fee_fn: &dyn Fn(AuthCtx, Tx, TxOutcome) -> Result<Response, E>,
    ctx_ptr: usize,
    tx_ptr: usize,
    outcome_ptr: usize,
) -> usize
where
    E: Display,
{
    let ctx_bytes = unsafe { Region::consume(ctx_ptr as *mut Region) };
    let tx_bytes = unsafe { Region::consume(tx_ptr as *mut Region) };
    let outcome_bytes = unsafe { Region::consume(outcome_ptr as *mut Region) };

    let res = (|| {
        let ctx: Context = unwrap_into_generic_result!(ctx_bytes.deserialize_borsh());
        let auth_ctx = make_auth_ctx!(ctx, &mut ExternalStorage, &ExternalApi, &ExternalQuerier);
        let tx = unwrap_into_generic_result!(tx_bytes.deserialize_borsh());
        let outcome = unwrap_into_generic_result!(outcome_bytes.deserialize_borsh());

        finalize_fee_fn(auth_ctx, tx, outcome).into_generic_result()
    })();

    let res_bytes = res.to_borsh_vec().unwrap();

    Region::release_buffer(res_bytes) as usize
}
