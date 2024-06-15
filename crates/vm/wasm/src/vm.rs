use {
    crate::{
        blake2b_512, blake2s_256, blake3, db_next, db_read, db_remove, db_scan, db_write, debug,
        ed25519_batch_verify, ed25519_verify, keccak256, query_chain, read_then_wipe,
        secp256k1_pubkey_recover, secp256k1_verify, secp256r1_verify, sha2_256, sha2_512,
        sha2_512_truncated, sha3_256, sha3_512, sha3_512_truncated, write_to_memory, Environment,
        VmError, VmResult,
    },
    grug_app::{PrefixStore, QueryProvider, Vm},
    grug_types::{to_borsh_vec, Context},
    wasmer::{
        imports, Function, FunctionEnv, Instance as WasmerInstance, Module, Singlepass, Store,
    },
};

pub struct WasmVm {
    _wasm_instance: Box<WasmerInstance>,
    wasm_store: Store,
    fe: FunctionEnv<Environment>,
}

impl Vm for WasmVm {
    type Error = VmError;
    type Program = Vec<u8>;

    fn build_instance(
        storage: PrefixStore,
        querier: QueryProvider<Self>,
        program: Vec<u8>,
    ) -> Result<Self, Self::Error> {
        // create Wasm store
        // for now we use the singlepass compiler
        let mut wasm_store = Store::new(Singlepass::default());

        // compile Wasm byte code into module
        let module = Module::new(&wasm_store, program)?;

        // create function environment and register imports
        // note: memory/store/instance in the env hasn't been set yet at this point
        let fe = FunctionEnv::new(&mut wasm_store, Environment::new(storage, querier));
        let import_obj = imports! {
            "env" => {
                "db_read"                  => Function::new_typed_with_env(&mut wasm_store, &fe, db_read),
                "db_scan"                  => Function::new_typed_with_env(&mut wasm_store, &fe, db_scan),
                "db_next"                  => Function::new_typed_with_env(&mut wasm_store, &fe, db_next),
                "db_write"                 => Function::new_typed_with_env(&mut wasm_store, &fe, db_write),
                "db_remove"                => Function::new_typed_with_env(&mut wasm_store, &fe, db_remove),
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
        let wasm_instance = WasmerInstance::new(&mut wasm_store, &module, &import_obj)?;
        let wasm_instance = Box::new(wasm_instance);

        // set memory/store/instance in the env
        let env = fe.as_mut(&mut wasm_store);
        env.set_memory(&wasm_instance)?;
        env.set_wasm_instance(wasm_instance.as_ref())?;

        Ok(Self {
            _wasm_instance: wasm_instance,
            wasm_store,
            fe,
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

        read_then_wipe(env, &mut wasm_store, res_ptr)
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

        read_then_wipe(env, &mut wasm_store, res_ptr)
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

        read_then_wipe(env, &mut wasm_store, res_ptr)
    }
}
