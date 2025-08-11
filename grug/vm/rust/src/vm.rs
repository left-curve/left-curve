use {
    crate::{ContractWrapper, VmError, VmResult, get_contract_impl},
    grug_app::{GasTracker, Instance, QuerierProvider, StorageProvider, Vm},
    grug_types::{BorshDeExt, BorshSerExt, Context, Hash256, MockApi},
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
        _code_hash: Hash256,
        storage: StorageProvider,
        // Rust VM doesn't need this "readonly" flag, because everything happens
        // in Rust, the compiler can prevent storage writes in query methods
        // (unlike Wasm VM where an FFI is involved).
        _storage_readonly: bool,
        querier: Box<dyn QuerierProvider>,
        // In Rust VM, we don't check for max query depth.
        _query_depth: usize,
        // Rust VM doesn't support gas tracking, so we make no use of the
        // provided `GasTracker`.
        _gas_tracker: GasTracker,
    ) -> VmResult<RustInstance> {
        Ok(RustInstance {
            storage,
            querier,
            wrapper: ContractWrapper::from_bytes(code),
        })
    }
}

pub struct RustInstance {
    storage: StorageProvider,
    querier: Box<dyn QuerierProvider>,
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
                res.to_borsh_vec()
            },
            "cron_execute" => {
                let res = contract.cron_execute(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                )?;
                res.to_borsh_vec()
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
                res.to_borsh_vec()
            },
            "execute" => {
                let res = contract.execute(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                res.to_borsh_vec()
            },
            "migrate" => {
                let res = contract.migrate(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                res.to_borsh_vec()
            },
            "query" => {
                let res = contract.query(
                    ctx.clone(),
                    &self.storage,
                    &MockApi,
                    &self.querier,
                    param.as_ref(),
                )?;
                res.to_borsh_vec()
            },
            "authenticate" => {
                let tx = param.deserialize_borsh()?;
                let res = contract.authenticate(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    tx,
                )?;
                res.to_borsh_vec()
            },
            "backrun" => {
                let tx = param.deserialize_borsh()?;
                let res = contract.backrun(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    tx,
                )?;
                res.to_borsh_vec()
            },
            "bank_execute" => {
                let msg = param.deserialize_borsh()?;
                let res = contract.bank_execute(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    msg,
                )?;
                res.to_borsh_vec()
            },
            "bank_query" => {
                let msg = param.deserialize_borsh()?;
                let res = contract.bank_query(
                    ctx.clone(),
                    &self.storage,
                    &MockApi,
                    &self.querier,
                    msg,
                )?;
                res.to_borsh_vec()
            },
            "withhold_fee" => {
                let tx = param.deserialize_borsh()?;
                let res = contract.withhold_fee(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    tx,
                )?;
                res.to_borsh_vec()
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
                let result = param2.deserialize_borsh()?;
                let res = contract.reply(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    param1.as_ref(),
                    result,
                )?;
                res.to_borsh_vec()
            },
            "finalize_fee" => {
                let tx = param1.deserialize_borsh()?;
                let outcome = param2.deserialize_borsh()?;
                let res = contract.finalize_fee(
                    ctx.clone(),
                    &mut self.storage,
                    &MockApi,
                    &self.querier,
                    tx,
                    outcome,
                )?;
                res.to_borsh_vec()
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
        grug_app::{GasTracker, Instance, QuerierProviderImpl, StorageProvider, Vm},
        grug_types::{
            Addr, Binary, BlockInfo, BorshSerExt, Coins, Context, Hash, JsonSerExt, MockStorage,
            Shared, Storage, Timestamp,
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
    fn calling_functions(name: &'static str, maybe_error: Option<&str>) {
        let code: Binary = ContractBuilder::new(Box::new(tester::instantiate))
            .build()
            .to_bytes()
            .into();

        let db = Shared::new(MockStorage::new());
        let mut vm = RustVm::new();

        let block = BlockInfo {
            height: 0,
            timestamp: Timestamp::from_nanos(0),
            hash: Hash::ZERO,
        };

        let gas_tracker = GasTracker::new_limitless();

        let querier_provider = QuerierProviderImpl::new_boxed(
            vm.clone(),
            Box::new(db.clone()),
            gas_tracker.clone(),
            block,
        );

        let storage_provider = StorageProvider::new(Box::new(db.clone()), &[b"tester"]);

        let instance = vm
            .build_instance(
                &code,
                Hash::ZERO,
                storage_provider,
                false,
                querier_provider,
                0,
                gas_tracker,
            )
            .unwrap();

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

        let result = instance.call_in_1_out_1(
            name,
            &ctx,
            &msg.to_json_value().unwrap().to_borsh_vec().unwrap(),
        );

        match maybe_error {
            // We expect the call to succeed. Check that the data is correctly
            // written to the DB.
            None => {
                assert!(result.is_ok());
                let value = db.read_access().read(b"testerlarry");
                assert_eq!(value, Some(b"engineer".to_vec()));
            },
            // We expect the call to fail. Check that the error message is as
            // expected.
            //
            // Here we have to compare the errors as strings, because we can't
            // derive `PartialEq` on `VmError`. This is because `VmError`
            // inherits `StdError`, which inherits `TryFromSliceError`, which
            // doesn't implement `PartialEq`.
            Some(expect) => {
                assert!(result.is_err_and(|actual| actual.to_string() == expect));
            },
        }
    }
}
