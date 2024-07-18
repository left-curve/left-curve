use {
    crate::{
        handle_submessages, AppError, AppResult, GasTracker, Instance, QuerierProvider,
        StorageProvider, Vm, CODES, CONTRACT_ADDRESS_KEY, CONTRACT_NAMESPACE,
    },
    grug_types::{
        from_json_slice, to_json_vec, Addr, BlockInfo, Context, Event, GenericResult, Hash,
        Response, Storage,
    },
    serde::{de::DeserializeOwned, ser::Serialize},
};

/// The response from a VM call, which can either be a result or a missing entry point.
/// This is used to handle the case where the entry point is missing
/// (e.g. `after_tx`, `receive`).
pub enum VmCallResponse<T> {
    MissingEntryPoint(String),
    Result(T),
}

impl<T> VmCallResponse<T> {
    pub fn map<T1>(self, f: impl FnOnce(T) -> T1) -> VmCallResponse<T1> {
        match self {
            VmCallResponse::Result(t) => VmCallResponse::Result(f(t)),

            VmCallResponse::MissingEntryPoint(entry_point) => {
                VmCallResponse::MissingEntryPoint(entry_point)
            },
        }
    }

    pub fn ok(ok: T) -> VmCallResponse<T> {
        VmCallResponse::Result(ok)
    }

    pub fn missing(entry_point: impl Into<String>) -> VmCallResponse<T> {
        VmCallResponse::MissingEntryPoint(entry_point.into())
    }

    /// Convert `VmCallResponse<T, E>` to `Result<T, E>`.
    /// - if the `entry point` is missing, return an `Ok(T)`.
    /// - if the `entry point` is not missing, return the `Err(AppError)`.
    pub fn ignore_missing_entry_point(self) -> Result<T, AppError> {
        match self {
            VmCallResponse::MissingEntryPoint(entry_point) => {
                Err(AppError::MissingEntryPoint { entry_point })
            },
            VmCallResponse::Result(result) => Ok(result),
        }
    }
}

impl<T> VmCallResponse<T>
where
    T: Transposable,
{
    /// Convert `VmCallResponse<Result<T, E>>` to `Result<VmCallResponse<T>, E>`.
    pub fn transpose(self) -> Result<VmCallResponse<T::T>, T::E> {
        match self {
            VmCallResponse::MissingEntryPoint(entry_point) => {
                Ok(VmCallResponse::MissingEntryPoint(entry_point))
            },
            VmCallResponse::Result(result) => match result.cast() {
                Ok(t) => Ok(VmCallResponse::Result(t)),
                Err(e) => Err(e),
            },
        }
    }
}

/// A trait for types that can be cast to a `Result`.
/// This is used to handle the case where [`VmCallResponse`] contains a result
/// and the result is an error.
///
/// With [transpose][VmCallResponse::transpose], allow to convert
///
/// `VmCallResponse<Result<T, E>>` to `Result<VmCallResponse<T>, E>`.
pub trait Transposable {
    type T;
    type E;
    fn cast(self) -> Result<Self::T, Self::E>;
}

impl<T, E> Transposable for Result<T, E> {
    type E = E;
    type T = T;

    fn cast(self) -> Result<T, E> {
        self
    }
}

/// Create a VM instance, and call a function that takes no input parameter and
/// returns one output.
pub fn call_in_0_out_1<VM, R>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: &Hash,
    ctx: &Context,
    storage_readonly: bool,
) -> AppResult<VmCallResponse<R>>
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
        ctx.block.clone(),
        &ctx.contract,
        code_hash,
        storage_readonly,
    )?;

    // Call the function; deserialize the output as JSON
    let out = instance
        .call_in_0_out_1(name, ctx)?
        .map(from_json_slice)
        .transpose()?;

    Ok(out)
}

/// Create a VM instance, and call a function that takes exactly one parameter
/// and returns one output.
pub fn call_in_1_out_1<VM, P, R>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: &Hash,
    ctx: &Context,
    storage_readonly: bool,
    param: &P,
) -> AppResult<VmCallResponse<R>>
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
        ctx.block.clone(),
        &ctx.contract,
        code_hash,
        storage_readonly,
    )?;

    // Serialize the param as JSON
    let param_raw = to_json_vec(param)?;

    // Call the function; deserialize the output as JSON
    let out = instance
        .call_in_1_out_1(name, ctx, &param_raw)?
        .map(from_json_slice)
        .transpose()?;

    Ok(out)
}

/// Create a VM instance, and call a function that takes exactly two parameters
/// and returns one output.
pub fn call_in_2_out_1<VM, P1, P2, R>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: &Hash,
    ctx: &Context,
    storage_readonly: bool,
    param1: &P1,
    param2: &P2,
) -> AppResult<VmCallResponse<R>>
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
        ctx.block.clone(),
        &ctx.contract,
        code_hash,
        storage_readonly,
    )?;

    // Serialize the params as JSON
    let param1_raw = to_json_vec(param1)?;
    let param2_raw = to_json_vec(param2)?;

    // Call the function; deserialize the output as JSON
    let out = instance
        .call_in_2_out_1(name, ctx, &param1_raw, &param2_raw)?
        .map(from_json_slice)
        .transpose()?;

    Ok(out)
}

/// Create a VM instance, call a function that takes exactly no input parameter
/// and returns [`Response`], and handle the submessages. Return a vector of
/// events emitted.
pub fn call_in_0_out_1_handle_response<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: &Hash,
    ctx: &Context,
    storage_readonly: bool,
) -> AppResult<VmCallResponse<Vec<Event>>>
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
    )?;

    let handled_response = response
        .map(|response| {
            handle_response(
                vm,
                storage,
                gas_tracker,
                name,
                ctx,
                response.into_std_result()?,
            )
        })
        .transpose()?;

    Ok(handled_response)

    // handle_response(vm, storage, gas_tracker, name, ctx, response)
}

/// Create a VM instance, call a function that takes exactly one parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_1_out_1_handle_response<VM, P>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: &Hash,
    ctx: &Context,
    storage_readonly: bool,
    param: &P,
) -> AppResult<VmCallResponse<Vec<Event>>>
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
        param,
    )?;

    let handled_response = response
        .map(|response| {
            handle_response(
                vm,
                storage,
                gas_tracker,
                name,
                ctx,
                response.into_std_result()?,
            )
        })
        .transpose()?;

    Ok(handled_response)
}

/// Create a VM instance, call a function that takes exactly two parameter and
/// returns [`Response`], and handle the submessages. Return a vector of events
/// emitted.
pub fn call_in_2_out_1_handle_response<VM, P1, P2>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    name: &'static str,
    code_hash: &Hash,
    ctx: &Context,
    storage_readonly: bool,
    param1: &P1,
    param2: &P2,
) -> AppResult<VmCallResponse<Vec<Event>>>
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
        param1,
        param2,
    )?;

    let handled_response = response
        .map(|response| {
            handle_response(
                vm,
                storage,
                gas_tracker,
                name,
                ctx,
                response.into_std_result()?,
            )
        })
        .transpose()?;

    Ok(handled_response)
}

fn create_vm_instance<VM>(
    mut vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    address: &Addr,
    code_hash: &Hash,
    storage_readonly: bool,
) -> AppResult<VM::Instance>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Load the program code from storage and deserialize
    let code = CODES.load(&storage, code_hash)?;

    // Create the providers
    let querier = QuerierProvider::new(vm.clone(), storage.clone(), gas_tracker.clone(), block);
    let storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, address]);

    Ok(vm.build_instance(
        &code,
        code_hash,
        storage,
        storage_readonly,
        querier,
        gas_tracker,
    )?)
}

pub(crate) fn handle_response<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
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
        .add_attribute(CONTRACT_ADDRESS_KEY, &ctx.contract)
        .add_attributes(response.attributes);

    // Handle submessages; append events emitted during submessage handling
    let mut events = vec![event];
    events.extend(handle_submessages(
        vm,
        storage,
        ctx.block.clone(),
        gas_tracker,
        ctx.contract.clone(),
        response.submsgs,
    )?);

    Ok(events)
}
