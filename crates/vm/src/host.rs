use {
    crate::Region,
    anyhow::{anyhow, bail, Context},
    cw_std::{from_json, Binary, ContractResult, Response},
    data_encoding::BASE64,
    std::cell::OnceCell,
    wasmi::{Caller, Extern, Instance, Memory, Store, TypedFunc, WasmParams, WasmResults},
};

pub struct Host<'a, S> {
    caller:     Caller<'a, S>,
    memory:     OnceCell<Memory>,
    alloc_fn:   OnceCell<TypedFunc<u32, u32>>,
    dealloc_fn: OnceCell<TypedFunc<u32, ()>>,
}

impl<'a, S> From<Caller<'a, S>> for Host<'a, S> {
    fn from(caller: Caller<'a, S>) -> Self {
        Self {
            caller,
            memory:     OnceCell::new(),
            alloc_fn:   OnceCell::new(),
            dealloc_fn: OnceCell::new(),
        }
    }
}

impl<'a, S> Host<'a, S> {
    pub fn new(instance: &Instance, store: &'a mut Store<S>) -> Self {
        Self {
            caller:     Caller::new(store, Some(instance)),
            memory:     OnceCell::new(),
            alloc_fn:   OnceCell::new(),
            dealloc_fn: OnceCell::new(),
        }
    }

    pub fn call_instantiate(&mut self, msg: impl AsRef<[u8]>) -> anyhow::Result<Response> {
        let res_bytes = self.call_entry_point_raw("instantiate", msg.as_ref())?;
        let res: ContractResult<Response> = from_json(res_bytes)?;
        res.into_result()
    }

    pub fn call_execute(&mut self, msg: impl AsRef<[u8]>) -> anyhow::Result<Response> {
        let res_bytes = self.call_entry_point_raw("execute", msg.as_ref())?;
        let res: ContractResult<Response> = from_json(res_bytes)?;
        res.into_result()
    }

    pub fn call_query(&mut self, msg: impl AsRef<[u8]>) -> anyhow::Result<Binary> {
        let res_bytes = self.call_entry_point_raw("query", msg.as_ref())?;
        let res: ContractResult<Binary> = from_json(res_bytes)?;
        res.into_result()
    }

    /// Call the contract's specified entry point, that takes exactly one input
    /// and returns one output.
    fn call_entry_point_raw(
        &mut self,
        name: &str,
        msg:  impl AsRef<[u8]>,
    ) -> anyhow::Result<Vec<u8>> {
        let msg_ptr = self.write_to_memory(msg.as_ref())?;
        let res_ptr: u32 = self.call(name, msg_ptr)?;
        self.read_then_wipe(res_ptr)
    }

    /// Get an immutable reference to the host state.
    pub(crate) fn data(&self) -> &S {
        self.caller.data()
    }

    /// Get a mutable reference to the host state.
    pub(crate) fn data_mut(&mut self) -> &mut S {
        self.caller.data_mut()
    }

    /// Call a function on the Wasm module.
    pub(crate) fn call<P, R>(&mut self, name: &str, params: P) -> anyhow::Result<R>
    where
        P: WasmParams,
        R: WasmResults,
    {
        self.get_typed_func(name)?
            .call(&mut self.caller, params)
            .map_err(Into::into)
    }

    /// Reserve a region in Wasm memory and write the given data into it.
    pub(crate) fn write_to_memory(&mut self, data: &[u8]) -> anyhow::Result<u32> {
        let region_ptr = self.alloc_fn().call(&mut self.caller, data.len() as u32)?;
        self.write_region(region_ptr, data)?;
        Ok(region_ptr)
    }

    /// Read data from a region in Wasm memory.
    pub(crate) fn read_from_memory(&self, region_ptr: u32) -> anyhow::Result<Vec<u8>> {
        let buf = self.read_memory(region_ptr as usize, Region::SIZE)?;
        let region = unsafe { Region::from_raw(&buf) };

        self.read_memory(region.offset as usize, region.length as usize)
    }

    /// Read data from a region then deallocate it. This is used almost
    /// exclusively for reading the response at the very end of the call.
    /// For all other use cases, Host::read_from_memory probably should be used.
    pub(crate) fn read_then_wipe(&mut self, region_ptr: u32) -> anyhow::Result<Vec<u8>> {
        let data = self.read_from_memory(region_ptr)?;
        self.dealloc_fn().call(&mut self.caller, region_ptr)?;
        Ok(data)
    }

    fn write_region(&mut self, region_ptr: u32, data: &[u8]) -> anyhow::Result<()> {
        let mut buf = self.read_memory(region_ptr as usize, Region::SIZE)?;
        let region = unsafe { Region::from_raw_mut(&mut buf) };
        // don't forget to update the Region length
        region.length = data.len() as u32;

        if region.length > region.capacity {
            bail!(
                "Region is too small! offset: {}, capacity: {}, data: {}",
                region.offset,
                region.capacity,
                BASE64.encode(data),
            );
        }

        self.write_memory(region.offset as usize, data)?;
        self.write_memory(region_ptr as usize, region.as_bytes())
    }

    fn read_memory(&self, offset: usize, length: usize) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![0x8; length];
        self.memory()
            .read(&self.caller, offset, &mut buf)
            .map(|_| buf)
            .map_err(|reason| anyhow!(
                "Failed to read from Wasm memory! offset: {}, length: {}, reason: {}",
                offset,
                length,
                reason,
            ))
    }

    fn write_memory(&mut self, offset: usize, data: &[u8]) -> anyhow::Result<()> {
        self.memory()
            .write(&mut self.caller, offset, data)
            .map_err(|reason| anyhow!(
                "Failed to write to Wasm memory! offset: {}, data: {}, reason: {}",
                offset,
                BASE64.encode(data),
                reason,
            ))
    }

    fn get_typed_func<P, R>(&self, name: &str) -> anyhow::Result<TypedFunc<P, R>>
    where
        P: WasmParams,
        R: WasmResults,
    {
        self.caller
            .get_export(name)
            .and_then(Extern::into_func)
            .with_context(|| format!("Can't find function `{name}` in Wasm exports"))?
            .typed(&self.caller)
            .map_err(Into::into)
    }

    fn get_memory(&self) -> anyhow::Result<Memory> {
        self.caller
            .get_export("memory")
            .and_then(Extern::into_memory)
            .context("Can't find memory in Wasm exports")
    }

    // TODO: switch to OnceCell::get_or_try_init once it's stablized:
    // https://github.com/rust-lang/rust/issues/109737
    fn memory(&self) -> Memory {
        *self.memory.get_or_init(|| self.get_memory().unwrap_or_else(|err| {
            panic!("[Host]: Failed to get memory: {err}");
        }))
    }

    fn alloc_fn(&self) -> TypedFunc<u32, u32> {
        *self.alloc_fn.get_or_init(|| self.get_typed_func("allocate").unwrap_or_else(|err| {
            panic!("[Host]: Failed to get `allocate` function: {err}");
        }))
    }

    fn dealloc_fn(&self) -> TypedFunc<u32, ()> {
        *self.dealloc_fn.get_or_init(|| self.get_typed_func("deallocate").unwrap_or_else(|err| {
            panic!("[Host]: Failed to get `deallocate` function: {err}");
        }))
    }
}
