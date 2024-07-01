use {
    crate::{
        handle_submessages, AppError, AppResult, SharedCacheVM, SharedGasTracker, Vm,
        CONTRACT_ADDRESS_KEY,
    },
    grug_types::{
        from_json_slice, to_json_vec, Context, Event, GenericResult, Hash, Response, Storage,
    },
    serde::{de::DeserializeOwned, ser::Serialize},
};

/// Create a VM instance, and call a function that takes no input parameter and
/// returns one output.
pub fn call_in_0_out_1<VM, R>(
    name: &'static str,
    storage: Box<dyn Storage>,
    code_hash: &Hash,
    ctx: &Context,
    gas_tracker: SharedGasTracker,
    cache_vm: SharedCacheVM<VM>,
) -> AppResult<R>
where
    R: DeserializeOwned,
    VM: Vm,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance: VM = cache_vm.build_instance(
        storage,
        ctx.block.clone(),
        &ctx.contract,
        code_hash,
        gas_tracker,
    )?;

    // Call the function; deserialize the output as JSON
    let data = instance.call_in_0_out_1(name, ctx)?;
    Ok(from_json_slice(data)?)
}

/// Create a VM instance, and call a function that takes exactly one parameter
/// and returns one output.
pub fn call_in_1_out_1<VM, P, R>(
    name: &'static str,
    storage: Box<dyn Storage>,
    code_hash: &Hash,
    ctx: &Context,
    gas_tracker: SharedGasTracker,
    cache_vm: SharedCacheVM<VM>,
    param: &P,
) -> AppResult<R>
where
    P: Serialize,
    R: DeserializeOwned,
    VM: Vm,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance: VM = cache_vm.build_instance(
        storage,
        ctx.block.clone(),
        &ctx.contract,
        code_hash,
        gas_tracker,
    )?;

    // Serialize the param as JSON
    let param_raw = to_json_vec(param)?;

    // Call the function; deserialize the output as JSON
    let data = instance.call_in_1_out_1(name, ctx, &param_raw)?;
    Ok(from_json_slice(data)?)
}

/// Create a VM instance, and call a function that takes exactly two parameters
/// and returns one output.
pub fn call_in_2_out_1<VM, P1, P2, R>(
    name: &'static str,
    storage: Box<dyn Storage>,
    code_hash: &Hash,
    ctx: &Context,
    gas_tracker: SharedGasTracker,
    cache_vm: SharedCacheVM<VM>,
    param1: &P1,
    param2: &P2,
) -> AppResult<R>
where
    P1: Serialize,
    P2: Serialize,
    R: DeserializeOwned,
    VM: Vm,
    AppError: From<VM::Error>,
{
    // Create the VM instance
    let instance: VM = cache_vm.build_instance(
        storage,
        ctx.block.clone(),
        &ctx.contract,
        code_hash,
        gas_tracker,
    )?;

    // Serialize the params as JSON
    let param1_raw = to_json_vec(param1)?;
    let param2_raw = to_json_vec(param2)?;

    // Call the function; deserialize the output as JSON
    let data = instance.call_in_2_out_1(name, ctx, &param1_raw, &param2_raw)?;
    Ok(from_json_slice(data)?)
}

/// Create a VM instance, call a function that takes exactly no input parameter
/// and returns [`Response`], and handle the submessages. Return a vector of
/// events emitted.
#[rustfmt::skip]
pub fn call_in_0_out_1_handle_response<VM>(
    name: &'static str,
    storage: Box<dyn Storage>,
    code_hash: &Hash,
    ctx: &Context,
    gas_tracker: SharedGasTracker,
    cache_vm: SharedCacheVM<VM>,
) -> AppResult<Vec<Event>>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    let response = call_in_0_out_1::<VM, GenericResult<Response>>(
        name,
        storage.clone(),
        code_hash,
        ctx,
        gas_tracker.clone(),
        cache_vm.clone(),
    )?
    .into_std_result()?;

    handle_response::<VM>(name, storage, ctx, gas_tracker,cache_vm, response)
}

/// Create a VM instance, call a function that takes exactly one parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_1_out_1_handle_response<VM, P>(
    name: &'static str,
    storage: Box<dyn Storage>,
    code_hash: &Hash,
    ctx: &Context,
    gas_tracker: SharedGasTracker,
    cache_vm: SharedCacheVM<VM>,
    param: &P,
) -> AppResult<Vec<Event>>
where
    P: Serialize,
    VM: Vm,
    AppError: From<VM::Error>,
{
    let response = call_in_1_out_1::<VM, _, GenericResult<Response>>(
        name,
        storage.clone(),
        code_hash,
        ctx,
        gas_tracker.clone(),
        cache_vm.clone(),
        param,
    )?
    .into_std_result()?;

    handle_response::<VM>(name, storage, ctx, gas_tracker, cache_vm, response)
}

/// Create a VM instance, call a function that takes exactly two parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_2_out_1_handle_response<VM, P1, P2>(
    name: &'static str,
    storage: Box<dyn Storage>,
    code_hash: &Hash,
    ctx: &Context,
    gas_tracker: SharedGasTracker,
    cache_vm: SharedCacheVM<VM>,
    param1: &P1,
    param2: &P2,
) -> AppResult<Vec<Event>>
where
    P1: Serialize,
    P2: Serialize,
    VM: Vm,
    AppError: From<VM::Error>,
{
    let response = call_in_2_out_1::<VM, _, _, GenericResult<Response>>(
        name,
        storage.clone(),
        code_hash,
        ctx,
        gas_tracker.clone(),
        cache_vm.clone(),
        param1,
        param2,
    )?
    .into_std_result()?;

    handle_response::<VM>(name, storage, ctx, gas_tracker, cache_vm, response)
}

pub(crate) fn handle_response<VM>(
    name: &'static str,
    storage: Box<dyn Storage>,
    ctx: &Context,
    gas_tracker: SharedGasTracker,
    cache_vm: SharedCacheVM<VM>,
    response: Response,
) -> AppResult<Vec<Event>>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    // Create an event for this call
    let event = Event::new(name)
        .add_attribute(CONTRACT_ADDRESS_KEY, &ctx.contract)
        .add_attributes(response.attributes);

    // Handle submessages; append events emitted during submessage handling
    let mut events = vec![event];
    events.extend(handle_submessages::<VM>(
        storage,
        ctx.block.clone(),
        gas_tracker,
        cache_vm,
        ctx.contract.clone(),
        response.submsgs,
    )?);

    Ok(events)
}
