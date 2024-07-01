use {
    crate::{
        blake2b_512, blake2s_256, blake3, db_next, db_next_key, db_next_value, db_read, db_remove,
        db_remove_range, db_scan, db_write, debug, ed25519_batch_verify, ed25519_verify, keccak256,
        query_chain, read_then_wipe, secp256k1_pubkey_recover, secp256k1_verify, secp256r1_verify,
        sha2_256, sha2_512, sha2_512_truncated, sha3_256, sha3_512, sha3_512_truncated,
        write_to_memory, Environment, VmError, VmResult,
    },
    grug_app::{PrefixStore, QueryProvider, SharedGasTracker, Vm, VmCacheSize},
    grug_types::{to_borsh_vec, Context, Size},
    std::sync::Arc,
    wasmer::{
        imports, CompilerConfig, Engine, Function, FunctionEnv, Instance as WasmerInstance, Module,
        Singlepass, Store,
    },
    wasmer_middlewares::{
        metering::{get_remaining_points, set_remaining_points, MeteringPoints},
        Metering,
    },
};

/// Gas cost per operation
///
/// TODO: Mocked to 1 now, need to be discussed
const GAS_PER_OPERATION: u64 = 1;

#[derive(Clone)]
pub struct WasmCache {
    pub module: Module,
    pub engine: Engine,
}

impl VmCacheSize for WasmCache {
    fn size(&self) -> usize {
        // Based on Cosmwasm implementation:
        // Some manual tests on Simon's machine showed that Engine is roughly 3-5 KB big,
        // so give it a constant 10 KiB estimate.
        Size::kibi(10).bytes()
    }
}

pub struct WasmVm {
    _wasm_instance: Box<WasmerInstance>,
    wasm_store: Store,
    fe: FunctionEnv<Environment>,
    gas_tracker: SharedGasTracker,
}

impl WasmVm {
    fn consume_gas(&mut self) -> VmResult<()> {
        match get_remaining_points(&mut self.wasm_store, &self._wasm_instance) {
            MeteringPoints::Remaining(remaining) => {
                // Reset gas consumed
                set_remaining_points(&mut self.wasm_store, &self._wasm_instance, u64::MAX);
                self.gas_tracker
                    .write_access()
                    .deduct(u64::MAX - remaining)?;
                Ok(())
            },
            MeteringPoints::Exhausted => {
                panic!("Out of gas, this should have been caught earlier!")
            },
        }
    }
}

impl Vm for WasmVm {
    type Cache = WasmCache;
    type Error = VmError;
    type Program = Vec<u8>;

    fn build_cache(program: Self::Program) -> Result<Self::Cache, Self::Error> {
        let mut compiler = Singlepass::new();
        let metering = Arc::new(Metering::new(u64::MAX, |_| GAS_PER_OPERATION));
        compiler.canonicalize_nans(true);
        compiler.push_middleware(metering);
        let engine: Engine = compiler.into();

        // compile Wasm byte code into module
        let now = std::time::Instant::now();
        let module = Module::new(&engine, program)?;
        tracing::debug!("Wasm compilation time: {:?}", now.elapsed());

        let size = std::mem::size_of_val(&module);
        tracing::debug!("Wasm module size: {}", size);
        let size = std::mem::size_of_val(&engine);
        tracing::debug!("Wasm engine size: {}", size);

        Ok(WasmCache { module, engine })
    }

    fn build_instance_from_cache(
        storage: PrefixStore,
        querier: QueryProvider<Self>,
        cache: Self::Cache,
        gas_tracker: SharedGasTracker,
    ) -> Result<Self, Self::Error> {
        let mut wasm_store = Store::new(cache.engine);

        let fe = FunctionEnv::new(
            &mut wasm_store,
            Environment::new(storage, querier, gas_tracker.clone()),
        );
        let import_obj = imports! {
            "env" => {
                "db_read"                  => Function::new_typed_with_env(&mut wasm_store, &fe, db_read),
                "db_scan"                  => Function::new_typed_with_env(&mut wasm_store, &fe, db_scan),
                "db_next"                  => Function::new_typed_with_env(&mut wasm_store, &fe, db_next),
                "db_next_key"              => Function::new_typed_with_env(&mut wasm_store, &fe, db_next_key),
                "db_next_value"            => Function::new_typed_with_env(&mut wasm_store, &fe, db_next_value),
                "db_write"                 => Function::new_typed_with_env(&mut wasm_store, &fe, db_write),
                "db_remove"                => Function::new_typed_with_env(&mut wasm_store, &fe, db_remove),
                "db_remove_range"          => Function::new_typed_with_env(&mut wasm_store, &fe, db_remove_range),
                "secp256k1_verify"         => Function::new_typed_with_env(&mut wasm_store, &fe, secp256k1_verify),
                "secp256r1_verify"         => Function::new_typed_with_env(&mut wasm_store, &fe, secp256r1_verify),
                "secp256k1_pubkey_recover" => Function::new_typed_with_env(&mut wasm_store, &fe, secp256k1_pubkey_recover),
                "ed25519_verify"           => Function::new_typed_with_env(&mut wasm_store, &fe, ed25519_verify),
                "ed25519_batch_verify"     => Function::new_typed_with_env(&mut wasm_store, &fe, ed25519_batch_verify),
                "sha2_256"                 => Function::new_typed_with_env(&mut wasm_store, &fe, sha2_256),
                "sha2_512"                 => Function::new_typed_with_env(&mut wasm_store, &fe, sha2_512),
                "sha2_512_truncated"       => Function::new_typed_with_env(&mut wasm_store, &fe, sha2_512_truncated),
                "sha3_256"                 => Function::new_typed_with_env(&mut wasm_store, &fe, sha3_256),
                "sha3_512"                 => Function::new_typed_with_env(&mut wasm_store, &fe, sha3_512),
                "sha3_512_truncated"       => Function::new_typed_with_env(&mut wasm_store, &fe, sha3_512_truncated),
                "keccak256"                => Function::new_typed_with_env(&mut wasm_store, &fe, keccak256),
                "blake2s_256"              => Function::new_typed_with_env(&mut wasm_store, &fe, blake2s_256),
                "blake2b_512"              => Function::new_typed_with_env(&mut wasm_store, &fe, blake2b_512),
                "blake3"                   => Function::new_typed_with_env(&mut wasm_store, &fe, blake3),
                "debug"                    => Function::new_typed_with_env(&mut wasm_store, &fe, debug),
                "query_chain"              => Function::new_typed_with_env(&mut wasm_store, &fe, query_chain),
            }
        };

        // create wasmer instance
        let wasm_instance = WasmerInstance::new(&mut wasm_store, &cache.module, &import_obj)?;
        let wasm_instance = Box::new(wasm_instance);

        // set memory/store/instance in the env
        let env = fe.as_mut(&mut wasm_store);
        env.set_memory(&wasm_instance)?;
        env.set_wasm_instance(wasm_instance.as_ref())?;

        Ok(Self {
            _wasm_instance: wasm_instance,
            wasm_store,
            fe,
            gas_tracker,
        })
    }

    fn call_in_0_out_1(mut self, name: &str, ctx: &Context) -> VmResult<Vec<u8>> {
        let mut fe_mut = self.fe.clone().into_mut(&mut self.wasm_store);
        let (env, mut wasm_store) = fe_mut.data_and_store_mut();

        let ctx_ptr = write_to_memory(env, &mut wasm_store, &to_borsh_vec(ctx)?)?;
        let res_ptr: u32 = env
            .call_function1(&mut wasm_store, name, &[ctx_ptr.into()])?
            .try_into()
            .map_err(VmError::ReturnType)?;

        let data = read_then_wipe(env, &mut wasm_store, res_ptr)?;

        self.consume_gas()?;

        Ok(data)
    }

    fn call_in_1_out_1<P>(mut self, name: &str, ctx: &Context, param: &P) -> VmResult<Vec<u8>>
    where
        P: AsRef<[u8]>,
    {
        let mut fe_mut = self.fe.clone().into_mut(&mut self.wasm_store);
        let (env, mut wasm_store) = fe_mut.data_and_store_mut();

        let ctx_ptr = write_to_memory(env, &mut wasm_store, &to_borsh_vec(ctx)?)?;
        let param1_ptr = write_to_memory(env, &mut wasm_store, param.as_ref())?;
        let res_ptr: u32 = env
            .call_function1(&mut wasm_store, name, &[ctx_ptr.into(), param1_ptr.into()])?
            .try_into()
            .map_err(VmError::ReturnType)?;

        let data = read_then_wipe(env, &mut wasm_store, res_ptr)?;

        self.consume_gas()?;

        Ok(data)
    }

    fn call_in_2_out_1<P1, P2>(
        mut self,
        name: &str,
        ctx: &Context,
        param1: &P1,
        param2: &P2,
    ) -> VmResult<Vec<u8>>
    where
        P1: AsRef<[u8]>,
        P2: AsRef<[u8]>,
    {
        let mut fe_mut = self.fe.clone().into_mut(&mut self.wasm_store);
        let (env, mut wasm_store) = fe_mut.data_and_store_mut();

        let ctx_ptr = write_to_memory(env, &mut wasm_store, &to_borsh_vec(ctx)?)?;
        let param1_ptr = write_to_memory(env, &mut wasm_store, param1.as_ref())?;
        let param2_ptr = write_to_memory(env, &mut wasm_store, param2.as_ref())?;
        let res_ptr: u32 = env
            .call_function1(&mut wasm_store, name, &[
                ctx_ptr.into(),
                param1_ptr.into(),
                param2_ptr.into(),
            ])?
            .try_into()
            .map_err(VmError::ReturnType)?;
        let data = read_then_wipe(env, &mut wasm_store, res_ptr)?;

        self.consume_gas()?;

        Ok(data)
    }
}
