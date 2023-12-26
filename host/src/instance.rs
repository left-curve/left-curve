use {
    crate::{Allocator, Memory},
    wasmi::{Store, WasmParams, WasmResults},
};

/// Wraps around the wasmi Instance and a Store, providing some convenience
/// methods.
pub struct Instance<HostState> {
    instance:  wasmi::Instance,
    store:     Store<HostState>,
    memory:    Memory,
    allocator: Allocator,
}

impl<HostState> Instance<HostState> {
    pub fn new(instance: wasmi::Instance, store: Store<HostState>) -> anyhow::Result<Self> {
        Ok(Self {
            allocator: (&instance, &store).try_into()?,
            memory:    (&instance, &store).try_into()?,
            store,
            instance,
        })
    }

    pub fn call<P, R>(&mut self, name: &str, params: P) -> anyhow::Result<R>
    where
        P: WasmParams,
        R: WasmResults,
    {
        // using get_types_func here probably has a bit of overhead over get_func
        // which we may consider optimizing if this code is to be used in production
        self.instance
            .get_typed_func(&self.store, name)?
            .call(&mut self.store, params)
            .map_err(Into::into)
    }

    pub fn release_buffer(&mut self, data: Vec<u8>) -> anyhow::Result<u32> {
        let region_ptr = self.allocator.allocate(&mut self.store, data.capacity())?;
        self.memory.write_region(&mut self.store, region_ptr, &data)?;
        Ok(region_ptr)
    }

    pub fn consume_region(&mut self, region_ptr: u32) -> anyhow::Result<Vec<u8>> {
        let data = self.memory.read_region(&self.store, region_ptr)?;
        self.allocator.deallocate(&mut self.store, region_ptr)?;
        Ok(data)
    }

    /// Consume the instance, return the host state.
    pub fn recycle(self) -> HostState {
        self.store.into_data()
    }
}
