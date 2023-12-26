mod builder;
mod region;

pub use {builder::InstanceBuilder, region::Region};

use {
    std::cell::OnceCell,
    wasmi::{
        core::HostError, errors::MemoryError, Caller, Extern, Instance, Memory, Store, TypedFunc,
        WasmParams, WasmResults,
    },
};

pub struct Host<'a, HostState> {
    caller:     Caller<'a, HostState>,
    memory:     OnceCell<Memory>,
    alloc_fn:   OnceCell<TypedFunc<u32, u32>>,
    dealloc_fn: OnceCell<TypedFunc<u32, ()>>,
}

impl<'a, HostState> From<Caller<'a, HostState>> for Host<'a, HostState> {
    fn from(caller: Caller<'a, HostState>) -> Self {
        Self {
            caller,
            memory:     OnceCell::new(),
            alloc_fn:   OnceCell::new(),
            dealloc_fn: OnceCell::new(),
        }
    }
}

impl<'a, HostState> Host<'a, HostState> {
    pub fn new(instance: &Instance, store: &'a mut Store<HostState>) -> Self {
        Self {
            caller:     Caller::new(store, Some(instance)),
            memory:     OnceCell::new(),
            alloc_fn:   OnceCell::new(),
            dealloc_fn: OnceCell::new(),
        }
    }

    /// Get an immutable reference to the host state.
    pub fn data(&self) -> &HostState {
        self.caller.data()
    }

    /// Get a mutable reference to the host state.
    pub fn data_mut(&mut self) -> &mut HostState {
        self.caller.data_mut()
    }

    /// Call a function on the Wasm module.
    pub fn call<Params, Results>(&mut self, name: &str, params: Params) -> Result<Results>
    where
        Params:  WasmParams,
        Results: WasmResults,
    {
        self.get_typed_func(name)?.call(&mut self.caller, params).map_err(Into::into)
    }

    /// Reserve a region in Wasm memory and write the given data into it.
    pub fn write_to_memory(&mut self, data: &[u8]) -> Result<u32> {
        let region_ptr = self.alloc_fn().call(&mut self.caller, data.len() as u32)?;
        self.write_region(region_ptr, data)?;
        Ok(region_ptr)
    }

    /// Read data from a region in Wasm memory.
    pub fn read_from_memory(&self, region_ptr: u32) -> Result<Vec<u8>> {
        let buf = self.read_memory(region_ptr as usize, Region::SIZE)?;
        let region = unsafe { Region::from_raw(&buf) };

        self.read_memory(region.offset as usize, region.length as usize)
    }

    /// Read data from a region then deallocate it. This is used almost
    /// exclusively for reading the response at the very end of the call.
    /// For all other use cases, Host::read_from_memory probably should be used.
    pub fn read_then_wipe(&mut self, region_ptr: u32) -> Result<Vec<u8>> {
        let data = self.read_from_memory(region_ptr)?;
        self.dealloc_fn().call(&mut self.caller, region_ptr)?;
        Ok(data)
    }

    fn write_region(&mut self, region_ptr: u32, data: &[u8]) -> Result<()> {
        let mut buf = self.read_memory(region_ptr as usize, Region::SIZE)?;
        let region = unsafe { Region::from_raw_mut(&mut buf) };
        // don't forget to update the Region length
        region.length = data.len() as u32;

        if region.length > region.capacity {
            return Err(Error::InsufficientRegion {
                capacity: region.capacity,
                length:   region.length,
            });
        }

        self.write_memory(region.offset as usize, data)?;
        self.write_memory(region_ptr as usize, region.as_bytes())
    }

    fn read_memory(&self, offset: usize, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0x8; length];
        self.memory()
            .read(&self.caller, offset, &mut buf)
            .map(|_| buf)
            .map_err(|reason| Error::ReadMemory {
                offset,
                length,
                reason,
            })
    }

    fn write_memory(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        self.memory()
            .write(&mut self.caller, offset, data)
            .map_err(|reason| Error::WriteMemory {
                offset,
                length: data.len(),
                reason,
            })
    }

    fn get_typed_func<Params, Results>(&self, name: &str, ) -> Result<TypedFunc<Params, Results>>
    where
        Params: WasmParams,
        Results: WasmResults,
    {
        self.caller
            .get_export(name)
            .and_then(Extern::into_func)
            .ok_or(Error::FunctionNotFound)?
            .typed(&self.caller)
            .map_err(Into::into)
    }

    fn get_memory(&self) -> Result<Memory> {
        self.caller
            .get_export("memory")
            .and_then(Extern::into_memory)
            .ok_or(Error::MemoryNotFound)
    }

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

// we can't use anyhow::Error, because it doesn't implement wasi::core::HostError
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Wasmi(#[from] wasmi::Error),

    #[error("Can't find memory in Wasm exports")]
    MemoryNotFound,

    #[error("Can't find the given function in Wasm exports")]
    FunctionNotFound,

    #[error("Region too small! capacity: {capacity}, attempting to write: {length}")]
    InsufficientRegion {
        capacity: u32,
        length:   u32,
    },

    #[error("Failed to read memory! offset: {offset}, length: {length}, reason: {reason}")]
    ReadMemory {
        offset: usize,
        length: usize,
        reason: MemoryError,
    },

    #[error("Failed to write to Wasm memory! offset: {offset}, length: {length}, reason: {reason}")]
    WriteMemory {
        offset: usize,
        length: usize,
        reason: MemoryError,
    },
}

impl From<Error> for wasmi::Error {
    fn from(err: Error) -> Self {
        wasmi::Error::host(err)
    }
}

impl HostError for Error {}

type Result<T> = std::result::Result<T, Error>;
