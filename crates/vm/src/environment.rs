use {
    crate::{Storage, VmError, VmResult},
    std::{
        borrow::{Borrow, BorrowMut},
        ptr::NonNull,
        sync::{Arc, RwLock},
    },
    wasmer::{AsStoreMut, AsStoreRef, Instance, Memory, MemoryView, Value},
};

// TODO: add explaination on why these fields need to be Options
pub struct ContextData<S> {
    store:         Option<S>,
    wasm_instance: Option<NonNull<Instance>>,
}

impl<S> ContextData<S> {
    pub fn new() -> Self {
        Self {
            store:         None,
            wasm_instance: None,
        }
    }
}

pub struct Environment<S> {
    memory: Option<Memory>,
    data:   Arc<RwLock<ContextData<S>>>,
}

unsafe impl<S> Send for Environment<S> {}

impl<S> Environment<S> {
    pub fn new() -> Self {
        Self {
            memory: None,
            data:   Arc::new(RwLock::new(ContextData::new())),
        }
    }

    pub fn memory<'a>(&self, wasm_store: &'a impl AsStoreRef) -> VmResult<MemoryView<'a>> {
        self.memory
            .as_ref()
            .ok_or(VmError::MemoryNotSet)
            .map(|mem| mem.view(wasm_store))
    }

    pub fn with_context_data<C, T>(&self, callback: C) -> VmResult<T>
    where
        C: FnOnce(&ContextData<S>) -> VmResult<T>,
    {
        let guard = self.data.read().map_err(|_| VmError::FailedReadLock)?;
        callback(guard.borrow())
    }

    pub fn with_context_data_mut<C, T>(&mut self, callback: C) -> VmResult<T>
    where
        C: FnOnce(&mut ContextData<S>) -> VmResult<T>,
    {
        let mut guard = self.data.write().map_err(|_| VmError::FailedWriteLock)?;
        callback(guard.borrow_mut())
    }

    pub fn with_wasm_instance<C, T>(&self, callback: C) -> VmResult<T>
    where
        C: FnOnce(&wasmer::Instance) -> VmResult<T>,
    {
        self.with_context_data(|ctx| {
            let instance_ptr = ctx.wasm_instance.ok_or(VmError::WasmerInstanceNotSet)?;
            let instance_ref = unsafe { instance_ptr.as_ref() };
            callback(instance_ref)
        })
    }

    pub fn set_memory(&mut self, wasm_instance: &Instance) -> VmResult<()> {
        let memory = wasm_instance.exports.get_memory("memory")?;
        self.memory = Some(memory.clone());
        Ok(())
    }

    pub fn set_store(&mut self, store: S) -> VmResult<()> {
        self.with_context_data_mut(|ctx| {
            ctx.store = Some(store);
            Ok(())
        })
    }

    pub fn set_wasm_instance(&mut self, wasm_instance: &Instance) -> VmResult<()> {
        self.with_context_data_mut(|ctx| {
            ctx.wasm_instance = Some(NonNull::from(wasm_instance));
            Ok(())
        })
    }

    pub fn take_store(&mut self) -> VmResult<S> {
        self.with_context_data_mut(|ctx| {
            ctx.store.take().ok_or(VmError::StoreNotSet)
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
        self.with_wasm_instance(|wasm_instance| {
            let func = wasm_instance.exports.get_function(name)?;
            func.call(wasm_store, args).map_err(Into::into)
        })
    }
}

impl<S: Storage> Environment<S> {
    pub fn with_store<C, T>(&self, callback: C) -> VmResult<T>
    where
        C: FnOnce(&dyn Storage) -> VmResult<T>,
    {
        self.with_context_data(|ctx| {
            let store = ctx.store.as_ref().ok_or(VmError::StoreNotSet)?;
            callback(store)
        })
    }

    pub fn with_store_mut<C, T>(&mut self, callback: C) -> VmResult<T>
    where
        C: FnOnce(&mut dyn Storage) -> VmResult<T>,
    {
        self.with_context_data_mut(|ctx| {
            let store = ctx.store.as_mut().ok_or(VmError::StoreNotSet)?;
            callback(store)
        })
    }
}
