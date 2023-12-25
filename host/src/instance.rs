use {
    crate::Region,
    anyhow::bail,
    std::mem::size_of,
    wasmi::{Memory, Store, WasmParams, WasmResults},
};

/// Wraps around the wasmi Instance and a Store, providing some convenience
/// methods.
pub struct Instance<T = ()> {
    pub instance: wasmi::Instance,
    pub store: Store<T>,
}

impl Instance {
    pub fn call<P, R>(&mut self, name: &str, params: P) -> anyhow::Result<R>
    where
        P: WasmParams,
        R: WasmResults,
    {
        self.instance
            .get_typed_func(&self.store, name)?
            .call(&mut self.store, params)
            .map_err(Into::into)
    }

    pub fn read_region(&self, region_ptr: u32) -> anyhow::Result<Vec<u8>> {
        let memory = self.memory();

        let buf = self.read_memory(memory, region_ptr as usize, size_of::<Region>())?;
        let region = Region::deserialize(&buf)?;

        self.read_memory(memory, region.offset as usize, region.length as usize)
    }

    pub fn write_region(&mut self, region_ptr: u32, data: &[u8]) -> anyhow::Result<()> {
        let memory = self.memory();

        let buf = self.read_memory(memory, region_ptr as usize, size_of::<Region>())?;
        let mut region = Region::deserialize(&buf)?;
        // don't forget to update the Region length
        region.length = data.len() as u32;

        if region.length > region.capacity {
            bail!(
                "Region too small! Capacity: {}, attempting to write: {}",
                region.capacity,
                region.length,
            );
        }

        self.write_memory(memory, region.offset as usize, data)?;
        self.write_memory(memory, region_ptr as usize, &region.serialize())
    }

    fn read_memory(&self, memory: Memory, offset: usize, len: usize) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![0x8; len];
        if let Err(err) = memory.read(&self.store, offset, &mut buf) {
            bail!(
                "Failed to read memory! offset: {}, length: {}, reason: {}",
                offset,
                len,
                err,
            );
        }
        Ok(buf)
    }

    fn write_memory(&mut self, memory: Memory, offset: usize, data: &[u8]) -> anyhow::Result<()> {
        if let Err(err) = memory.write(&mut self.store, offset, data) {
            bail!(
                "Failed to write to Wasm memory! offset: {}, length: {}, reason: {}",
                offset,
                data.len(),
                err,
            );
        }
        Ok(())
    }

    fn memory(&self) -> Memory {
        self.instance.get_memory(&self.store, "memory").expect("Memory not found")
    }
}
