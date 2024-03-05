use {
    crate::{
        db_next, db_read, db_remove, db_scan, db_write, debug, query_chain, read_then_wipe,
        secp256k1_verify, secp256r1_verify, write_to_memory, BackendQuerier, BackendStorage,
        Environment, VmError, VmResult,
    },
    cw_std::{
        from_json, to_borsh, to_json, BankQueryMsg, BankQueryResponse, Binary, Context,
        GenericResult, Response, SubMsgResult, TransferMsg, Tx,
    },
    wasmer::{
        imports, Function, FunctionEnv, Instance as WasmerInstance, Module, Singlepass, Store,
    },
};

pub struct Instance<S, Q> {
    _wasm_instance: Box<WasmerInstance>,
    wasm_store: Store,
    fe: FunctionEnv<Environment<S, Q>>,
}

impl<S, Q> Instance<S, Q>
where
    S: BackendStorage + 'static,
    Q: BackendQuerier + 'static,
{
    pub fn build_from_code(store: S, querier: Q, wasm_byte_code: &[u8]) -> VmResult<Self> {
        // create Wasm store
        // for now we use the singlepass compiler
        let mut wasm_store = Store::new(Singlepass::default());

        // compile Wasm byte code into module
        let module = Module::new(&wasm_store, wasm_byte_code)?;

        // create function environment and register imports
        // note: memory/store/instance in the env hasn't been set yet at this point
        let fe = FunctionEnv::new(&mut wasm_store, Environment::new(store, querier));
        let import_obj = imports! {
            "env" => {
                "db_read" => Function::new_typed_with_env(&mut wasm_store, &fe, db_read),
                "db_scan" => Function::new_typed_with_env(&mut wasm_store, &fe, db_scan),
                "db_next" => Function::new_typed_with_env(&mut wasm_store, &fe, db_next),
                "db_write" => Function::new_typed_with_env(&mut wasm_store, &fe, db_write),
                "db_remove" => Function::new_typed_with_env(&mut wasm_store, &fe, db_remove),
                "debug" => Function::new_typed_with_env(&mut wasm_store, &fe, debug),
                "query_chain" => Function::new_typed_with_env(&mut wasm_store, &fe, query_chain),
                "secp256k1_verify" => Function::new_typed_with_env(&mut wasm_store, &fe, secp256k1_verify),
                "secp256r1_verify" => Function::new_typed_with_env(&mut wasm_store, &fe, secp256r1_verify)
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

    pub fn call_instantiate(
        &mut self,
        ctx: &Context,
        msg: impl AsRef<[u8]>,
    ) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_1_out_1("instantiate", ctx, msg)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_execute(
        &mut self,
        ctx: &Context,
        msg: impl AsRef<[u8]>,
    ) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_1_out_1("execute", ctx, msg)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_query(
        &mut self,
        ctx: &Context,
        msg: impl AsRef<[u8]>,
    ) -> VmResult<GenericResult<Binary>> {
        let res_bytes = self.call_in_1_out_1("query", ctx, msg)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_migrate(
        &mut self,
        ctx: &Context,
        msg: impl AsRef<[u8]>,
    ) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_1_out_1("migrate", ctx, msg)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_reply(
        &mut self,
        ctx: &Context,
        msg: impl AsRef<[u8]>,
        events: &SubMsgResult,
    ) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_2_out_1("reply", ctx, msg, to_json(events)?)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_receive(&mut self, ctx: &Context) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_0_out_1("receive", ctx)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_before_block(&mut self, ctx: &Context) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_0_out_1("before_block", ctx)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_after_block(&mut self, ctx: &Context) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_0_out_1("after_block", ctx)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_before_tx(&mut self, ctx: &Context, tx: &Tx) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_1_out_1("before_tx", ctx, to_json(tx)?)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_after_tx(&mut self, ctx: &Context, tx: &Tx) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_1_out_1("after_tx", ctx, to_json(tx)?)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_bank_transfer(
        &mut self,
        ctx: &Context,
        msg: &TransferMsg,
    ) -> VmResult<GenericResult<Response>> {
        let res_bytes = self.call_in_1_out_1("bank_transfer", ctx, to_json(msg)?)?;
        from_json(res_bytes).map_err(Into::into)
    }

    pub fn call_bank_query(
        &mut self,
        ctx: &Context,
        msg: &BankQueryMsg,
    ) -> VmResult<GenericResult<BankQueryResponse>> {
        let res_bytes = self.call_in_1_out_1("bank_query", ctx, to_json(msg)?)?;
        from_json(res_bytes).map_err(Into::into)
    }

    /// Call a Wasm export function that takes exactly 0 input parameter (other
    /// than the context) and produces exactly 1 output.
    fn call_in_0_out_1(&mut self, name: &str, ctx: &Context) -> VmResult<Vec<u8>> {
        let mut fe_mut = self.fe.clone().into_mut(&mut self.wasm_store);
        let (env, mut wasm_store) = fe_mut.data_and_store_mut();

        let ctx_ptr = write_to_memory(env, &mut wasm_store, &to_borsh(ctx)?)?;
        let res_ptr: u32 = env
            .call_function1(&mut wasm_store, name, &[ctx_ptr.into()])?
            .try_into()
            .map_err(VmError::ReturnType)?;

        read_then_wipe(env, &mut wasm_store, res_ptr)
    }

    /// Call a Wasm export function that takes exactly 1 input parameter (other
    /// than the context) and produces exactly 1 output.
    fn call_in_1_out_1(
        &mut self,
        name: &str,
        ctx: &Context,
        param1: impl AsRef<[u8]>,
    ) -> VmResult<Vec<u8>> {
        let mut fe_mut = self.fe.clone().into_mut(&mut self.wasm_store);
        let (env, mut wasm_store) = fe_mut.data_and_store_mut();

        let ctx_ptr = write_to_memory(env, &mut wasm_store, &to_borsh(ctx)?)?;
        let param1_ptr = write_to_memory(env, &mut wasm_store, param1.as_ref())?;
        let res_ptr: u32 = env
            .call_function1(&mut wasm_store, name, &[ctx_ptr.into(), param1_ptr.into()])?
            .try_into()
            .map_err(VmError::ReturnType)?;

        read_then_wipe(env, &mut wasm_store, res_ptr)
    }

    /// Call a Wasm export function that takes exactly 2 input parameters (other
    /// than the context) and produces exactly 1 output.
    fn call_in_2_out_1(
        &mut self,
        name: &str,
        ctx: &Context,
        param1: impl AsRef<[u8]>,
        param2: impl AsRef<[u8]>,
    ) -> VmResult<Vec<u8>> {
        let mut fe_mut = self.fe.clone().into_mut(&mut self.wasm_store);
        let (env, mut wasm_store) = fe_mut.data_and_store_mut();

        let ctx_ptr = write_to_memory(env, &mut wasm_store, &to_borsh(ctx)?)?;
        let param1_ptr = write_to_memory(env, &mut wasm_store, param1.as_ref())?;
        let param2_ptr = write_to_memory(env, &mut wasm_store, param2.as_ref())?;
        let res_ptr: u32 = env
            .call_function1(
                &mut wasm_store,
                name,
                &[ctx_ptr.into(), param1_ptr.into(), param2_ptr.into()],
            )?
            .try_into()
            .map_err(VmError::ReturnType)?;

        read_then_wipe(env, &mut wasm_store, res_ptr)
    }
}
