use {
    crate::{VmError, VmResult},
    cw_std::{GenericResult, QueryRequest, QueryResponse},
    std::{
        borrow::{Borrow, BorrowMut},
        ptr::NonNull,
        sync::{Arc, RwLock},
    },
    wasmer::{AsStoreMut, AsStoreRef, Instance, Memory, MemoryView, Value},
};

pub trait BackendQuerier {
    fn query_chain(&self, req: QueryRequest) -> VmResult<GenericResult<QueryResponse>>;
}

// TODO: add explaination on why these fields need to be Options
#[derive(Default, Debug)]
pub struct ContextData<S, Q> {
    pub store:     S,
    pub querier:   Q,
    wasm_instance: Option<NonNull<Instance>>,
}

impl<S, Q> ContextData<S, Q> {
    pub fn new(store: S, querier: Q) -> Self {
        Self {
            store,
            querier,
            wasm_instance: None,
        }
    }
}

#[derive(Default, Debug)]
pub struct Environment<S, Q> {
    memory: Option<Memory>,
    data:   Arc<RwLock<ContextData<S, Q>>>,
}

unsafe impl<S, Q> Send for Environment<S, Q> {}

impl<S, Q> Environment<S, Q> {
    pub fn new(store: S, querier: Q) -> Self {
        Self {
            memory: None,
            data:   Arc::new(RwLock::new(ContextData::new(store, querier))),
        }
    }

    pub fn memory<'a>(&self, wasm_store: &'a impl AsStoreRef) -> VmResult<MemoryView<'a>> {
        self.memory
            .as_ref()
            .ok_or(VmError::MemoryNotSet)
            .map(|mem| mem.view(wasm_store))
    }

    pub fn with_context_data<C, T, E>(&self, callback: C) -> VmResult<T>
    where
        C: FnOnce(&ContextData<S, Q>) -> Result<T, E>,
        E: Into<VmError>,
    {
        let guard = self.data.read().map_err(|_| VmError::FailedReadLock)?;
        callback(guard.borrow()).map_err(Into::into)
    }

    pub fn with_context_data_mut<C, T, E>(&mut self, callback: C) -> VmResult<T>
    where
        C: FnOnce(&mut ContextData<S, Q>) -> Result<T, E>,
        E: Into<VmError>,
    {
        let mut guard = self.data.write().map_err(|_| VmError::FailedWriteLock)?;
        callback(guard.borrow_mut()).map_err(Into::into)
    }

    pub fn with_wasm_instance<C, T, E>(&self, callback: C) -> VmResult<T>
    where
        C: FnOnce(&wasmer::Instance) -> Result<T, E>,
        E: Into<VmError>,
    {
        self.with_context_data(|ctx| {
            let instance_ptr = ctx.wasm_instance.ok_or(VmError::WasmerInstanceNotSet)?;
            let instance_ref = unsafe { instance_ptr.as_ref() };
            callback(instance_ref).map_err(Into::into)
        })
    }

    pub fn set_memory(&mut self, wasm_instance: &Instance) -> VmResult<()> {
        let memory = wasm_instance.exports.get_memory("memory")?;
        self.memory = Some(memory.clone());
        Ok(())
    }

    pub fn set_wasm_instance(&mut self, wasm_instance: &Instance) -> VmResult<()> {
        self.with_context_data_mut(|ctx| -> VmResult<_> {
            ctx.wasm_instance = Some(NonNull::from(wasm_instance));
            Ok(())
        })
    }

    pub fn call_function1(
        &self,
        wasm_store: &mut impl AsStoreMut,
        name:       &str,
        args:       &[Value],
    ) -> VmResult<Value> {
        let ret = self.call_function(wasm_store, name, args)?;
        if ret.len() != 1 {
            return Err(VmError::ReturnCount {
                name:   name.into(),
                expect: 1,
                actual: ret.len(),
            });
        }
        Ok(ret[0].clone())
    }

    pub fn call_function0(
        &self,
        wasm_store: &mut impl AsStoreMut,
        name:       &str,
        args:       &[Value],
    ) -> VmResult<()> {
        let ret = self.call_function(wasm_store, name, args)?;
        if ret.len() != 0 {
            return Err(VmError::ReturnCount {
                name:   name.into(),
                expect: 0,
                actual: ret.len(),
            });
        }
        Ok(())
    }

    fn call_function(
        &self,
        wasm_store: &mut impl AsStoreMut,
        name:       &str,
        args:       &[Value],
    ) -> VmResult<Box<[Value]>> {
        // note: calling with_wasm_instance creates a read lock on the
        // ContextData. we must drop this lock before calling the function,
        // otherwise we get a deadlock (calling require a write lock which has
        // to wait for the previous read lock being dropped)
        let func = self.with_wasm_instance(|wasm_instance| -> VmResult<_> {
            let f = wasm_instance.exports.get_function(name)?;
            Ok(f.clone())
        })?;

        func.call(wasm_store, args).map_err(Into::into)
    }
}
