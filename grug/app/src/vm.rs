use {
    crate::{
        catch_event, handle_submessages, AppCtx, AppError, AppResult, EventResult, Instance,
        QuerierProvider, StorageProvider, Vm, CODES, CONTRACT_NAMESPACE,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{
        Addr, BorshDeExt, BorshSerExt, Context, EvtGuest, GenericResult, Hash256, Response,
    },
};

/// Create a VM instance, and call a function that takes no input parameter and
/// returns one output.
pub fn call_in_0_out_1<VM, R>(
    app_ctx: AppCtx<VM>,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
) -> AppResult<R>
where
    R: BorshDeserialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance =
        create_vm_instance(app_ctx, query_depth, state_mutable, ctx.contract, code_hash)?;

    // Call the function; deserialize the output as Borsh.
    let out_raw = instance.call_in_0_out_1(name, ctx)?;
    let out = out_raw.deserialize_borsh()?;

    Ok(out)
}

/// Create a VM instance, and call a function that takes exactly one parameter
/// and returns one output.
pub fn call_in_1_out_1<VM, P, R>(
    app_ctx: AppCtx<VM>,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    param: &P,
) -> AppResult<R>
where
    P: BorshSerialize,
    R: BorshDeserialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance =
        create_vm_instance(app_ctx, query_depth, state_mutable, ctx.contract, code_hash)?;

    // Serialize the param as Borsh.
    let param_raw = param.to_borsh_vec()?;

    // Call the function; deserialize the output as Borsh.
    let out_raw = instance.call_in_1_out_1(name, ctx, &param_raw)?;
    let out = out_raw.deserialize_borsh()?;

    Ok(out)
}

/// Create a VM instance, and call a function that takes exactly two parameters
/// and returns one output.
pub fn call_in_2_out_1<VM, P1, P2, R>(
    app_ctx: AppCtx<VM>,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    param1: &P1,
    param2: &P2,
) -> AppResult<R>
where
    P1: BorshSerialize,
    P2: BorshSerialize,
    R: BorshDeserialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance =
        create_vm_instance(app_ctx, query_depth, state_mutable, ctx.contract, code_hash)?;

    // Serialize the params as Borsh.
    let param1_raw = param1.to_borsh_vec()?;
    let param2_raw = param2.to_borsh_vec()?;

    // Call the function; deserialize the output as Borsh.
    let out_raw = instance.call_in_2_out_1(name, ctx, &param1_raw, &param2_raw)?;
    let out = out_raw.deserialize_borsh()?;

    Ok(out)
}

/// Create a VM instance, call a function that takes exactly no input parameter
/// and returns [`Response`], and handle the submessages. Return a vector of
/// events emitted.
pub fn call_in_0_out_1_handle_response<VM>(
    app_ctx: AppCtx<VM>,
    msg_depth: usize,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
) -> EventResult<EvtGuest>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = EvtGuest::base(ctx.contract, name);

    let response = catch_event! {
        evt,
        {
            call_in_0_out_1::<_, GenericResult<Response>>(
                app_ctx.clone(),
                query_depth,
                state_mutable,
                name,
                code_hash,
                ctx,
            )?
            .map_err(|msg| AppError::Guest {
                address: ctx.contract,
                name,
                msg,
            })
        }
    };

    handle_response(app_ctx, msg_depth, ctx, response, evt)
}

/// Create a VM instance, call a function that takes exactly one parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_1_out_1_handle_response<VM, P>(
    app_ctx: AppCtx<VM>,
    msg_depth: usize,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    param: &P,
) -> EventResult<EvtGuest>
where
    P: BorshSerialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = EvtGuest::base(ctx.contract, name);

    let response = catch_event! {
        evt,
        {
            call_in_1_out_1::<_, _, GenericResult<Response>>(
                app_ctx.clone(),
                query_depth,
                state_mutable,
                name,
                code_hash,
                ctx,
                param,
            )?
            .map_err(|msg| AppError::Guest {
                address: ctx.contract,
                name,
                msg,
            })
        }
    };

    handle_response(app_ctx, msg_depth, ctx, response, evt)
}

/// Create a VM instance, call a function that takes exactly two parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_2_out_1_handle_response<VM, P1, P2>(
    app_ctx: AppCtx<VM>,
    msg_depth: usize,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    param1: &P1,
    param2: &P2,
) -> EventResult<EvtGuest>
where
    P1: BorshSerialize,
    P2: BorshSerialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = EvtGuest::base(ctx.contract, name);

    let response = catch_event! {
        evt,
        {
            call_in_2_out_1::<_, _, _, GenericResult<Response>>(
                app_ctx.clone(),
                query_depth,
                state_mutable,
                name,
                code_hash,
                ctx,
                param1,
                param2
            )?
            .map_err(|msg| AppError::Guest {
                address: ctx.contract,
                name,
                msg,
            })
        }
    };

    handle_response(app_ctx, msg_depth, ctx, response, evt)
}

fn create_vm_instance<VM>(
    mut ctx: AppCtx<VM>,
    query_depth: usize,
    state_mutable: bool,
    contract: Addr,
    code_hash: Hash256,
) -> AppResult<VM::Instance>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Load the program code from storage and deserialize
    let code = CODES.load(&ctx.storage, code_hash)?;

    // Create the providers
    let querier = QuerierProvider::new(ctx.clone());
    let storage = StorageProvider::new(ctx.storage.clone(), &[CONTRACT_NAMESPACE, &contract]);

    Ok(ctx.vm.build_instance(
        &code.code,
        code_hash,
        storage,
        state_mutable,
        querier,
        query_depth,
        ctx.gas_tracker,
    )?)
}

pub(crate) fn handle_response<VM>(
    app_ctx: AppCtx<VM>,
    msg_depth: usize,
    ctx: &Context,
    response: Response,
    mut evt: EvtGuest,
) -> EventResult<EvtGuest>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    evt.contract_events = response.subevents;

    // Handle submessages; append events emitted during submessage handling
    handle_submessages(app_ctx, msg_depth, ctx.contract, response.submsgs).map_merge(
        evt,
        |subevents, mut evt| {
            evt.sub_events = subevents;
            evt
        },
    )
}
