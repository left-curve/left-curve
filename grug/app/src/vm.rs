use {
    crate::{
        handle_submessages, AppError, AppResult, GasTracker, Instance, QuerierProvider,
        StorageProvider, Vm, CODES, CONTRACT_ADDRESS_KEY, CONTRACT_NAMESPACE,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{
        Addr, BlockInfo, BorshDeExt, BorshSerExt, Config, Context, Event, GenericResult, Hash256,
        Response, Storage,
    },
};

/// Create a VM instance, and call a function that takes no input parameter and
/// returns one output.
pub fn call_in_0_out_1<VM, R>(
    vm: VM,
    cfg: Config,
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
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        cfg,
        storage,
        gas_tracker,
        query_depth,
        state_mutable,
        ctx.block,
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
    cfg: Config,
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
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        cfg,
        storage,
        gas_tracker,
        query_depth,
        state_mutable,
        ctx.block,
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
    cfg: Config,
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
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        cfg,
        storage,
        gas_tracker,
        query_depth,
        state_mutable,
        ctx.block,
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
    cfg: &Config,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let response = call_in_0_out_1::<_, GenericResult<Response>>(
        vm.clone(),
        cfg.clone(),
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
    })?;

    handle_response(
        vm,
        cfg,
        storage,
        gas_tracker,
        msg_depth,
        name,
        ctx,
        response,
    )
}

/// Create a VM instance, call a function that takes exactly one parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_1_out_1_handle_response<VM, P>(
    vm: VM,
    cfg: &Config,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    query_depth: usize,
    state_mutable: bool,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    param: &P,
) -> AppResult<Vec<Event>>
where
    P: BorshSerialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let response = call_in_1_out_1::<_, _, GenericResult<Response>>(
        vm.clone(),
        cfg.clone(),
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
    })?;

    handle_response(
        vm,
        cfg,
        storage,
        gas_tracker,
        msg_depth,
        name,
        ctx,
        response,
    )
}

/// Create a VM instance, call a function that takes exactly two parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_2_out_1_handle_response<VM, P1, P2>(
    vm: VM,
    cfg: &Config,
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
) -> AppResult<Vec<Event>>
where
    P1: BorshSerialize,
    P2: BorshSerialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let response = call_in_2_out_1::<_, _, _, GenericResult<Response>>(
        vm.clone(),
        cfg.clone(),
        storage.clone(),
        gas_tracker.clone(),
        query_depth,
        state_mutable,
        name,
        code_hash,
        ctx,
        param1,
        param2,
    )?
    .map_err(|msg| AppError::Guest {
        address: ctx.contract,
        name,
        msg,
    })?;

    handle_response(
        vm,
        cfg,
        storage,
        gas_tracker,
        msg_depth,
        name,
        ctx,
        response,
    )
}

fn create_vm_instance<VM>(
    mut vm: VM,
    cfg: Config,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    query_depth: usize,
    state_mutable: bool,
    block: BlockInfo,
    contract: Addr,
    code_hash: Hash256,
) -> AppResult<VM::Instance>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Load the program code from storage and deserialize
    let code = CODES.load(&storage, code_hash)?;

    // Create the providers
    let querier =
        QuerierProvider::new(vm.clone(), cfg, storage.clone(), gas_tracker.clone(), block);
    let storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &contract]);

    Ok(vm.build_instance(
        &code,
        code_hash,
        storage,
        state_mutable,
        querier,
        query_depth,
        gas_tracker,
    )?)
}

pub(crate) fn handle_response<VM>(
    vm: VM,
    cfg: &Config,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    name: &'static str,
    ctx: &Context,
    response: Response,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create an event for this call
    let event = Event::new(name)
        .add_attribute(CONTRACT_ADDRESS_KEY, ctx.contract)
        .add_attributes(response.attributes);

    // Handle submessages; append events emitted during submessage handling
    let mut events = vec![event];
    events.extend(handle_submessages(
        vm,
        cfg,
        storage,
        ctx.block,
        gas_tracker,
        msg_depth,
        ctx.contract,
        response.submsgs,
    )?);

    Ok(events)
}
