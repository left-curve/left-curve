use {
    crate::{
        catch_event, handle_submessages, AppError, AppResult, EventResult, GasTracker, Instance,
        QuerierProviderImpl, StorageProvider, Vm, CODES, CONTRACT_NAMESPACE,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{
        Addr, AuthResponse, BlockInfo, BorshDeExt, BorshSerExt, CheckedContractEvent, Context,
        EvtGuest, GenericResult, Hash256, Response, Storage,
    },
};

/// Create a VM instance, and call a function that takes no input parameter and
/// returns one output.
pub fn call_in_0_out_1<VM, R>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
) -> AppResult<R>
where
    R: BorshDeserialize,
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        storage,
        gas_tracker,
        ctx.block,
        query_depth,
        state_mutable,
        ctx.contract,
        code_hash,
    )?;

    // Call the function; deserialize the output as Borsh.
    let out_raw = instance.call_in_0_out_1(name, ctx)?;
    let out = out_raw.deserialize_borsh()?;

    Ok(out)
}

/// Create a VM instance, and call a function that takes exactly one parameter
/// and returns one output.
pub fn call_in_1_out_1<VM, P, R>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
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
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        storage,
        gas_tracker,
        ctx.block,
        query_depth,
        state_mutable,
        ctx.contract,
        code_hash,
    )?;

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
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
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
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        storage,
        gas_tracker,
        ctx.block,
        query_depth,
        state_mutable,
        ctx.contract,
        code_hash,
    )?;

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
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
) -> EventResult<EvtGuest>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = EvtGuest::base(ctx.contract, name);

    let response = catch_event! {
        {
            call_in_0_out_1::<_, GenericResult<Response>>(
                vm.clone(),
            storage.clone(),
            gas_tracker.clone(),
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
        },
        evt
    };

    handle_response(vm, storage, gas_tracker, msg_depth, ctx, response, evt)
}

/// Create a VM instance, call a function that takes exactly one parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_1_out_1_handle_response<VM, P>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
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
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = EvtGuest::base(ctx.contract, name);

    let response = catch_event! {
        {
            call_in_1_out_1::<_, _, GenericResult<Response>>(
                vm.clone(),
                storage.clone(),
                gas_tracker.clone(),
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
        },
        evt
    };

    handle_response(vm, storage, gas_tracker, msg_depth, ctx, response, evt)
}

pub fn call_in_1_out_1_handle_auth_response<VM, P>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    param: &P,
    backrun: &mut bool,
) -> EventResult<EvtGuest>
where
    P: BorshSerialize,
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = EvtGuest::base(ctx.contract, name);

    let auth_response = catch_event! {
        {
            call_in_1_out_1::<_, _, GenericResult<AuthResponse>>(
                vm.clone(),
                storage.clone(),
                gas_tracker.clone(),
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
        },
        evt
    };

    *backrun = auth_response.request_backrun;

    handle_response(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        ctx,
        auth_response.response,
        evt,
    )
}

/// Create a VM instance, call a function that takes exactly two parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_2_out_1_handle_response<VM, P1, P2>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
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
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = EvtGuest::base(ctx.contract, name);

    let response = catch_event! {
        {
            call_in_2_out_1::<_, _, _, GenericResult<Response>>(
                vm.clone(),
                storage.clone(),
                gas_tracker.clone(),
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
        },
        evt
    };

    handle_response(vm, storage, gas_tracker, msg_depth, ctx, response, evt)
}

fn create_vm_instance<VM>(
    mut vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    query_depth: usize,
    state_mutable: bool,
    contract: Addr,
    code_hash: Hash256,
) -> AppResult<VM::Instance>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    // Load the program code from storage and deserialize
    let code = CODES.load(&storage, code_hash)?;

    // Create the providers
    let querier = Box::new(QuerierProviderImpl::new(
        vm.clone(),
        storage.clone(),
        gas_tracker.clone(),
        block,
    ));
    let storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &contract]);

    Ok(vm.build_instance(
        &code.code,
        code_hash,
        storage,
        state_mutable,
        querier,
        query_depth,
        gas_tracker,
    )?)
}

fn handle_response<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    ctx: &Context,
    response: Response,
    mut evt: EvtGuest,
) -> EventResult<EvtGuest>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    evt.contract_events = response
        .subevents
        .into_iter()
        .map(|e| CheckedContractEvent {
            contract: evt.contract,
            ty: e.ty,
            data: e.data,
        })
        .collect();

    // Handle submessages; append events emitted during submessage handling
    handle_submessages(
        vm,
        storage,
        gas_tracker,
        ctx.block,
        msg_depth,
        ctx.contract,
        response.submsgs,
    )
    .map_merge(evt, |subevents, mut evt| {
        evt.sub_events = subevents;
        evt
    })
}
