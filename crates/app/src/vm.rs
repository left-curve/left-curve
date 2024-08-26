use {
    crate::{
        handle_submessages, AppError, AppResult, GasTracker, Instance, QuerierProvider,
        StorageProvider, Vm, CODES, CONTRACT_ADDRESS_KEY, CONTRACT_NAMESPACE,
    },
    grug_types::{
        Addr, BlockInfo, Context, Event, GenericResult, Hash256, JsonDeExt, JsonSerExt, Response,
        Storage,
    },
    serde::{de::DeserializeOwned, ser::Serialize},
};

/// Create a VM instance, and call a function that takes no input parameter and
/// returns one output.
pub fn call_in_0_out_1<VM, R>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    storage_readonly: bool,
    query_depth: usize,
) -> AppResult<R>
where
    R: DeserializeOwned,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        storage,
        gas_tracker,
        ctx.block,
        ctx.contract,
        code_hash,
        storage_readonly,
        query_depth,
    )?;

    // Call the function; deserialize the output as JSON
    let out_raw = instance.call_in_0_out_1(name, ctx)?;
    let out = out_raw.deserialize_json()?;

    Ok(out)
}

/// Create a VM instance, and call a function that takes exactly one parameter
/// and returns one output.
pub fn call_in_1_out_1<VM, P, R>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    storage_readonly: bool,
    query_depth: usize,
    param: &P,
) -> AppResult<R>
where
    P: Serialize,
    R: DeserializeOwned,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        storage,
        gas_tracker,
        ctx.block,
        ctx.contract,
        code_hash,
        storage_readonly,
        query_depth,
    )?;

    // Serialize the param as JSON
    let param_raw = param.to_json_vec()?;

    // Call the function; deserialize the output as JSON
    let out_raw = instance.call_in_1_out_1(name, ctx, &param_raw)?;
    let out = out_raw.deserialize_json()?;

    Ok(out)
}

/// Create a VM instance, and call a function that takes exactly two parameters
/// and returns one output.
pub fn call_in_2_out_1<VM, P1, P2, R>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    storage_readonly: bool,
    query_depth: usize,
    param1: &P1,
    param2: &P2,
) -> AppResult<R>
where
    P1: Serialize,
    P2: Serialize,
    R: DeserializeOwned,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance = create_vm_instance(
        vm,
        storage,
        gas_tracker,
        ctx.block,
        ctx.contract,
        code_hash,
        storage_readonly,
        query_depth,
    )?;

    // Serialize the params as JSON
    let param1_raw = param1.to_json_vec()?;
    let param2_raw = param2.to_json_vec()?;

    // Call the function; deserialize the output as JSON
    let out_raw = instance.call_in_2_out_1(name, ctx, &param1_raw, &param2_raw)?;
    let out = out_raw.deserialize_json()?;

    Ok(out)
}

/// Create a VM instance, call a function that takes exactly no input parameter
/// and returns [`Response`], and handle the submessages. Return a vector of
/// events emitted.
pub fn call_in_0_out_1_handle_response<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    response_depth: usize,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    storage_readonly: bool,
    query_depth: usize,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let response = call_in_0_out_1::<_, GenericResult<Response>>(
        vm.clone(),
        storage.clone(),
        gas_tracker.clone(),
        name,
        code_hash,
        ctx,
        storage_readonly,
        query_depth,
    )?
    .into_std_result()?;

    handle_response(
        vm,
        storage,
        gas_tracker,
        response_depth,
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
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    response_depth: usize,

    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    storage_readonly: bool,
    query_depth: usize,
    param: &P,
) -> AppResult<Vec<Event>>
where
    P: Serialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let response = call_in_1_out_1::<_, _, GenericResult<Response>>(
        vm.clone(),
        storage.clone(),
        gas_tracker.clone(),
        name,
        code_hash,
        ctx,
        storage_readonly,
        query_depth,
        param,
    )?
    .into_std_result()?;

    handle_response(
        vm,
        storage,
        gas_tracker,
        response_depth,
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
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    response_depth: usize,
    name: &'static str,
    code_hash: Hash256,
    ctx: &Context,
    storage_readonly: bool,
    query_depth: usize,
    param1: &P1,
    param2: &P2,
) -> AppResult<Vec<Event>>
where
    P1: Serialize,
    P2: Serialize,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let response = call_in_2_out_1::<_, _, _, GenericResult<Response>>(
        vm.clone(),
        storage.clone(),
        gas_tracker.clone(),
        name,
        code_hash,
        ctx,
        storage_readonly,
        query_depth,
        param1,
        param2,
    )?
    .into_std_result()?;

    handle_response(
        vm,
        storage,
        gas_tracker,
        response_depth,
        name,
        ctx,
        response,
    )
}

fn create_vm_instance<VM>(
    mut vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    address: Addr,
    code_hash: Hash256,
    storage_readonly: bool,
    query_depth: usize,
) -> AppResult<VM::Instance>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Load the program code from storage and deserialize
    let code = CODES.load(&storage, code_hash)?;

    // Create the providers
    let querier = QuerierProvider::new(vm.clone(), storage.clone(), gas_tracker.clone(), block);
    let storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &address]);

    Ok(vm.build_instance(
        &code,
        code_hash,
        storage,
        storage_readonly,
        querier,
        query_depth,
        gas_tracker,
    )?)
}

pub(crate) fn handle_response<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    response_depth: usize,
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
        storage,
        ctx.block,
        gas_tracker,
        response_depth,
        ctx.contract,
        response.submsgs,
    )?);

    Ok(events)
}
