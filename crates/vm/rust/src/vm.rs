use {
    crate::{get_contract_impl, ContractWrapper, InternalApi, VmError, VmResult},
    grug_app::{GasTracker, Instance, QuerierProvider, StorageProvider, Vm},
    grug_types::{from_borsh_slice, to_borsh_vec, Context, Hash256},
};

/// Names of export functions supported by Grug.
///
/// This doesn't include `allocate` and `deallocate`, which are only relevant
/// for the `WasmVm`.
pub const KNOWN_FUNCTIONS: [&str; 13] = [
    "instantate",
    "execute",
    "migrate",
    "receive",
    "reply",
    "query",
    "authenticate",
    "backrun",
    "bank_execute",
    "bank_query",
    "withhold_fee",
    "finalize_fee",
    "cron_execute",
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
        _code_hash: &Hash256,
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
                let res = contract.receive(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                )?;
                to_borsh_vec(&res)
            },
            "cron_execute" => {
                let res = contract.cron_execute(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                )?;
                to_borsh_vec(&res)
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
                    &InternalApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                to_borsh_vec(&res)
            },
            "execute" => {
                let res = contract.execute(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                to_borsh_vec(&res)
            },
            "migrate" => {
                let res = contract.migrate(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                to_borsh_vec(&res)
            },
            "query" => {
                let res = contract.query(
                    ctx.clone(),
                    &self.storage,
                    &InternalApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                to_borsh_vec(&res)
            },
            "authenticate" => {
                let tx = from_borsh_slice(param)?;
                let res = contract.authenticate(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                    tx,
                )?;
                to_borsh_vec(&res)
            },
            "backrun" => {
                let tx = from_borsh_slice(param)?;
                let res = contract.backrun(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                    tx,
                )?;
                to_borsh_vec(&res)
            },
            "bank_execute" => {
                let msg = from_borsh_slice(param)?;
                let res = contract.bank_execute(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                    msg,
                )?;
                to_borsh_vec(&res)
            },
            "bank_query" => {
                let msg = from_borsh_slice(param)?;
                let res = contract.bank_query(
                    ctx.clone(),
                    &self.storage,
                    &InternalApi,
                    &self.querier,
                    msg,
                )?;
                to_borsh_vec(&res)
            },
            "withhold_fee" => {
                let tx = from_borsh_slice(param)?;
                let res = contract.withhold_fee(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                    tx,
                )?;
                to_borsh_vec(&res)
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
                let submsg_res = from_borsh_slice(param2)?;
                let res = contract.reply(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                    param1.as_ref(),
                    submsg_res,
                )?;
                to_borsh_vec(&res)
            },
            "finalize_fee" => {
                let tx = from_borsh_slice(param1)?;
                let outcome = from_borsh_slice(param2)?;
                let res = contract.finalize_fee(
                    ctx.clone(),
                    &mut self.storage,
                    &InternalApi,
                    &self.querier,
                    tx,
                    outcome,
                )?;
                to_borsh_vec(&res)
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{ContractBuilder, RustVm},
        anyhow::ensure,
        grug_app::{GasTracker, Instance, QuerierProvider, Shared, StorageProvider, Vm},
        grug_types::{
            to_borsh_vec, to_json_value, Addr, Binary, BlockInfo, Coins, Context, Hash,
            MockStorage, NumberConst, Storage, Timestamp, Uint64,
        },
        test_case::test_case,
    };

    mod tester {
        use {
            grug_types::{MutableCtx, Response, StdResult},
            serde::{Deserialize, Serialize},
        };

        #[derive(Serialize, Deserialize)]
        pub struct InstantiateMsg {
            pub k: String,
            pub v: String,
        }

        pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
            ctx.storage.write(msg.k.as_bytes(), msg.v.as_bytes());

            Ok(Response::new())
        }
    }

    #[test_case(
        "instantiate",
        None;
        "known method that exists"
    )]
    #[test_case(
        "execute",
        Some("contract does not implement function `execute`");
        "known method that doesn't exist"
    )]
    #[test_case(
        "your_mom",
        Some("unknown function: `your_mom`");
        "unknown method"
    )]
    fn calling_functions(name: &'static str, maybe_error: Option<&str>) -> anyhow::Result<()> {
        let code: Binary = ContractBuilder::new(Box::new(tester::instantiate))
            .build()
            .into();

        let db = Shared::new(MockStorage::new());
        let mut vm = RustVm::new();

        let block = BlockInfo {
            height: Uint64::ZERO,
            timestamp: Timestamp::from_nanos(0),
            hash: Hash::ZERO,
        };

        let gas_tracker = GasTracker::new_limitless();

        let querier_provider = QuerierProvider::new(
            vm.clone(),
            Box::new(db.clone()),
            gas_tracker.clone(),
            block.clone(),
        );

        let storage_provider = StorageProvider::new(Box::new(db.clone()), &[b"tester"]);

        let instance = vm.build_instance(
            &code,
            &Hash::ZERO,
            storage_provider,
            false,
            querier_provider,
            gas_tracker,
        )?;

        let ctx = Context {
            chain_id: "dev-1".to_string(),
            block,
            contract: Addr::mock(1),
            sender: Some(Addr::mock(2)),
            funds: Some(Coins::new()),
            mode: None,
        };

        let msg = tester::InstantiateMsg {
            k: "larry".to_string(),
            v: "engineer".to_string(),
        };

        let result = instance.call_in_1_out_1(name, &ctx, &to_borsh_vec(&to_json_value(&msg)?)?);

        match maybe_error {
            // We expect the call to succeed. Check that the data is correctly
            // written to the DB.
            None => {
                ensure!(result.is_ok());
                let value = db.read_access().read(b"testerlarry");
                ensure!(value == Some(b"engineer".to_vec()));
            },
            // We expect the call to fail. Check that the error message is as
            // expected.
            //
            // Here we have to compare the errors as strings, because we can't
            // derive `PartialEq` on `VmError`. This is because `VmError`
            // inherits `StdError`, which inherits `TryFromSliceError`, which
            // doesn't implement `PartialEq`.
            Some(expect) => {
                ensure!(result.is_err_and(|actual| actual.to_string() == expect));
            },
        }

        Ok(())
    }
}
