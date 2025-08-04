use {
    grug_app::{AppError, GasTracker, Instance, QuerierProvider, StorageProvider, Vm},
    grug_types::{Backtraceable, Context, Hash256},
    grug_vm_rust::{RustInstance, RustVm},
    grug_vm_wasm::{WasmInstance, WasmVm},
    std::collections::HashSet,
};

#[grug_macros::backtrace]
pub enum VmError {
    #[error("RustVm error: {0}")]
    Rust(grug_vm_rust::VmError),

    #[error("WasmVm error: {0}")]
    Wasm(grug_vm_wasm::VmError),
}

impl From<VmError> for AppError {
    fn from(err: VmError) -> Self {
        let err = err.into_generic_backtraced_error();
        AppError::Vm {
            error: err.error,
            backtrace: err.backtrace,
        }
    }
}

#[derive(Clone)]
pub struct HybridVm {
    pub rust: RustVm,
    pub wasm: WasmVm,
    /// A set of code hashes that will be run in the Rust VM.
    /// Any code hash that's not in this set will be run in the Wasm VM.
    pub code_hashes_for_rust: HashSet<Hash256>,
}

impl HybridVm {
    pub fn new<T>(wasm_cache_capacity: usize, code_hashes_for_rust: T) -> Self
    where
        T: Into<HashSet<Hash256>>,
    {
        Self {
            rust: RustVm::new(),
            wasm: WasmVm::new(wasm_cache_capacity),
            code_hashes_for_rust: code_hashes_for_rust.into(),
        }
    }
}

impl Vm for HybridVm {
    type Error = VmError;
    type Instance = HybridInstance;

    fn build_instance(
        &mut self,
        code: &[u8],
        code_hash: Hash256,
        storage: StorageProvider,
        state_mutable: bool,
        querier: Box<dyn QuerierProvider>,
        query_depth: usize,
        gas_tracker: GasTracker,
    ) -> Result<Self::Instance, Self::Error> {
        if self.code_hashes_for_rust.contains(&code_hash) {
            let instance = self.rust.build_instance(
                code,
                code_hash,
                storage,
                state_mutable,
                querier,
                query_depth,
                gas_tracker,
            )?;
            Ok(HybridInstance::Rust(instance))
        } else {
            let instance = self.wasm.build_instance(
                code,
                code_hash,
                storage,
                state_mutable,
                querier,
                query_depth,
                gas_tracker,
            )?;
            Ok(HybridInstance::Wasm(instance))
        }
    }
}

pub enum HybridInstance {
    Rust(RustInstance),
    Wasm(WasmInstance),
}

impl Instance for HybridInstance {
    type Error = VmError;

    fn call_in_0_out_1(self, name: &'static str, ctx: &Context) -> Result<Vec<u8>, Self::Error> {
        match self {
            HybridInstance::Rust(instance) => {
                let res = instance.call_in_0_out_1(name, ctx)?;
                Ok(res)
            },
            HybridInstance::Wasm(instance) => {
                let res = instance.call_in_0_out_1(name, ctx)?;
                Ok(res)
            },
        }
    }

    fn call_in_1_out_1<P>(
        self,
        name: &'static str,
        ctx: &Context,
        param: &P,
    ) -> Result<Vec<u8>, Self::Error>
    where
        P: AsRef<[u8]>,
    {
        match self {
            HybridInstance::Rust(instance) => {
                let res = instance.call_in_1_out_1(name, ctx, param)?;
                Ok(res)
            },
            HybridInstance::Wasm(instance) => {
                let res = instance.call_in_1_out_1(name, ctx, param)?;
                Ok(res)
            },
        }
    }

    fn call_in_2_out_1<P1, P2>(
        self,
        name: &'static str,
        ctx: &Context,
        param1: &P1,
        param2: &P2,
    ) -> Result<Vec<u8>, Self::Error>
    where
        P1: AsRef<[u8]>,
        P2: AsRef<[u8]>,
    {
        match self {
            HybridInstance::Rust(instance) => {
                let res = instance.call_in_2_out_1(name, ctx, param1, param2)?;
                Ok(res)
            },
            HybridInstance::Wasm(instance) => {
                let res = instance.call_in_2_out_1(name, ctx, param1, param2)?;
                Ok(res)
            },
        }
    }
}
