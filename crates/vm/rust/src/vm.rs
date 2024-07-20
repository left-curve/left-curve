use {
    crate::{get_contract_impl, ContractWrapper, VmError, VmResult},
    grug_app::{GasTracker, Instance, QuerierProvider, StorageProvider, Vm},
    grug_types::{from_json_slice, to_json_vec, Context, Hash, MockApi},
};

/// Names of export functions supported by Grug.
///
/// This doesn't include `allocate` and `deallocate`, which are only relevant
/// for the `WasmVm`.
pub const KNOWN_FUNCTIONS: [&str; 11] = [
    "instantate",
    "execute",
    "migrate",
    "receive",
    "reply",
    "query",
    "before_tx",
    "after_tx",
    "bank_execute",
    "bank_query",
    "cron_execute",
    // TODO: add taxman and IBC entry points
];

#[derive(Default, Clone)]
pub struct RustVm;

impl RustVm {
    pub fn new() -> Self {
        Self
    }
}

impl Vm for RustVm {
    type Error = VmError;
    type Instance = RustInstance;

    fn build_instance(
        &mut self,
        code: &[u8],
        _code_hash: &Hash,
        storage: StorageProvider,
        // Rust VM doesn't need this "readonly" flag, because everything happens
        // in Rust, the compiler can prevent storage writes in query methods
        // (unlike Wasm VM where an FFI is involved).
        _storage_readonly: bool,
        querier: QuerierProvider<Self>,
        // Rust VM doesn't support gas tracking, so we make no use of the
        // provided `GasTracker`.
        _gas_tracker: GasTracker,
    ) -> VmResult<RustInstance> {
        Ok(RustInstance {
            storage,
            querier,
            wrapper: ContractWrapper::from(code),
        })
    }
}

pub struct RustInstance {
    storage: StorageProvider,
    querier: QuerierProvider<RustVm>,
    wrapper: ContractWrapper,
}

impl Instance for RustInstance {
    type Error = VmError;

    fn call_in_0_out_1(mut self, name: &'static str, ctx: &Context) -> VmResult<Vec<u8>> {
        let contract = get_contract_impl(self.wrapper)?;
        match name {
            "receive" => {
                let res =
                    contract.receive(ctx.clone(), &mut self.storage, &MockApi, &self.querier)?;
                to_json_vec(&res)
            },
            "cron_execute" => {
                let res = contract.cron_execute(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                )?;
                to_json_vec(&res)
            },
            _ if KNOWN_FUNCTIONS.contains(&name) => {
                return Err(VmError::incorrect_number_of_inputs(name, 0));
            },
            _ => {
                return Err(VmError::unknown_function(name));
            },
        }
        .map_err(Into::into)
    }

    fn call_in_1_out_1<P>(
        mut self,
        name: &'static str,
        ctx: &Context,
        param: &P,
    ) -> VmResult<Vec<u8>>
    where
        P: AsRef<[u8]>,
    {
        let contract = get_contract_impl(self.wrapper)?;
        match name {
            "instantiate" => {
                let res = contract.instantiate(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                to_json_vec(&res)
            },
            "execute" => {
                let res = contract.execute(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                to_json_vec(&res)
            },
            "migrate" => {
                let res = contract.migrate(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                to_json_vec(&res)
            },
            "query" => {
                let res = contract.query(
                    ctx.clone(),
                    &self.storage,
                    &MockApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                to_json_vec(&res)
            },
            "before_tx" => {
                let tx = from_json_slice(param)?;
                let res = contract.before_tx(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    tx,
                )?;
                to_json_vec(&res)
            },
            "after_tx" => {
                let tx = from_json_slice(param)?;
                let res = contract.after_tx(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    tx,
                )?;
                to_json_vec(&res)
            },
            "bank_execute" => {
                let msg = from_json_slice(param)?;
                let res = contract.bank_execute(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    msg,
                )?;
                to_json_vec(&res)
            },
            "bank_query" => {
                let msg = from_json_slice(param)?;
                let res = contract.bank_query(
                    ctx.clone(),
                    &self.storage,
                    &MockApi,
                    &self.querier,
                    msg,
                )?;
                to_json_vec(&res)
            },
            _ if KNOWN_FUNCTIONS.contains(&name) => {
                return Err(VmError::incorrect_number_of_inputs(name, 1));
            },
            _ => {
                return Err(VmError::unknown_function(name));
            },
        }
        .map_err(Into::into)
    }

    fn call_in_2_out_1<P1, P2>(
        mut self,
        name: &'static str,
        ctx: &Context,
        param1: &P1,
        param2: &P2,
    ) -> VmResult<Vec<u8>>
    where
        P1: AsRef<[u8]>,
        P2: AsRef<[u8]>,
    {
        let contract = get_contract_impl(self.wrapper)?;
        match name {
            "reply" => {
                let submsg_res = from_json_slice(param2)?;
                let res = contract.reply(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    param1.as_ref(),
                    submsg_res,
                )?;
                to_json_vec(&res)
            },
            _ if KNOWN_FUNCTIONS.contains(&name) => {
                return Err(VmError::incorrect_number_of_inputs(name, 2));
            },
            _ => {
                return Err(VmError::unknown_function(name));
            },
        }
        .map_err(Into::into)
    }
}
