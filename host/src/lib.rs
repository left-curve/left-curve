use {
    anyhow::Context,
    std::{cell::OnceCell, fs::File, mem::size_of, path::Path},
    wasmi::{
        core::HostError, errors::MemoryError, Caller, Engine, Extern, Instance, IntoFunc, Linker,
        Memory, Module, Store, TypedFunc, WasmParams, WasmResults,
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

    pub fn data(&self) -> &HostState {
        self.caller.data()
    }

    pub fn data_mut(&mut self) -> &mut HostState {
        self.caller.data_mut()
    }

    pub fn call<Params, Results>(&mut self, name: &str, params: Params) -> Result<Results, Error>
    where
        Params:  WasmParams,
        Results: WasmResults,
    {
        self.get_function(name)?.call(&mut self.caller, params).map_err(Into::into)
    }

    pub fn release_buffer(&mut self, data: Vec<u8>) -> Result<u32, Error> {
        let region_ptr = self.alloc_fn().call(&mut self.caller, data.capacity() as u32)?;
        self.write_region(region_ptr, &data)?;
        Ok(region_ptr)
    }

    pub fn consume_region(&mut self, region_ptr: u32) -> Result<Vec<u8>, Error> {
        let data = self.read_region(region_ptr)?;
        self.dealloc_fn().call(&mut self.caller, region_ptr).map_err(Error::from)?;
        Ok(data)
    }

    pub fn read_region(&self, region_ptr: u32) -> Result<Vec<u8>, Error> {
        let buf = self.read_memory(region_ptr as usize, size_of::<Region>())?;
        let region = Region::deserialize(&buf)?;

        self.read_memory(region.offset as usize, region.length as usize)
    }

    fn write_region(
        &mut self,
        region_ptr: u32,
        data: &[u8],
    ) -> Result<(), Error> {
        let buf = self.read_memory(region_ptr as usize, size_of::<Region>())?;
        let mut region = Region::deserialize(&buf)?;
        // don't forget to update the Region length
        region.length = data.len() as u32;

        if region.length > region.capacity {
            return Err(Error::InsufficientRegion {
                capacity: region.capacity,
                length:   region.length,
            });
        }

        self.write_memory(region.offset as usize, data)?;
        self.write_memory(region_ptr as usize, &region.serialize())
    }

    fn read_memory(&self, offset: usize, length: usize) -> Result<Vec<u8>, Error> {
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

    fn write_memory(&mut self, offset: usize, data: &[u8]) -> Result<(), Error> {
        self.memory()
            .write(&mut self.caller, offset, data)
            .map_err(|reason| Error::WriteMemory {
                offset,
                length: data.len(),
                reason,
            })
    }

    fn get_function<Params, Results>(&self, name: &str) -> Result<TypedFunc<Params, Results>, Error>
    where
        Params:  WasmParams,
        Results: WasmResults,
    {
        self.caller
            .get_export(name)
            .and_then(Extern::into_func)
            .ok_or(Error::FunctionNotFound)?
            .typed(&self.caller)
            .map_err(Into::into)
    }

    fn get_memory(&self) -> Result<Memory, Error> {
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
        *self.alloc_fn.get_or_init(|| self.get_function("allocate").unwrap_or_else(|err| {
            panic!("[Host]: Failed to get `allocate` function: {err}");
        }))
    }

    fn dealloc_fn(&self) -> TypedFunc<u32, ()> {
        *self.dealloc_fn.get_or_init(|| self.get_function("deallocate").unwrap_or_else(|err| {
            panic!("[Host]: Failed to get `deallocate` function: {err}");
        }))
    }
}

// ---------------------------------- builder ----------------------------------

#[derive(Default)]
pub struct InstanceBuilder<HostState> {
    engine: Engine,
    module: Option<Module>,
    store:  Option<Store<HostState>>,
    linker: Option<Linker<HostState>>,
}

impl<HostState> InstanceBuilder<HostState> {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            module: None,
            store:  None,
            linker: None,
        }
    }

    pub fn with_wasm_file(mut self, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = File::open(path)?;
        self.module = Some(Module::new(&self.engine, &mut file)?);
        Ok(self)
    }

    pub fn with_host_state(mut self, data: HostState) -> Self {
        self.store = Some(Store::new(&self.engine, data));
        self.linker = Some(Linker::new(&self.engine));
        self
    }

    pub fn with_host_function<Params, Results>(
        mut self,
        name: &str,
        func: impl IntoFunc<HostState, Params, Results>,
    ) -> anyhow::Result<Self> {
        let mut linker = self.take_linker()?;
        linker.func_wrap("env", name, func)?;
        self.linker = Some(linker);

        Ok(self)
    }

    pub fn finalize(mut self) -> anyhow::Result<(Instance, Store<HostState>)> {
        let module = self.take_module()?;
        let mut store = self.take_store()?;
        let linker = self.take_linker()?;
        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

        Ok((instance, store))
    }

    fn take_module(&mut self) -> anyhow::Result<Module> {
        self.module.take().context("Module not yet initialized")
    }

    fn take_store(&mut self) -> anyhow::Result<Store<HostState>> {
        self.store.take().context("Store not yet initialized")
    }

    fn take_linker(&mut self) -> anyhow::Result<Linker<HostState>> {
        self.linker.take().context("Linker not yet initialized")
    }
}

// ---------------------------------- region -----------------------------------

/// Similar to sdk::Region
struct Region {
    pub offset:   u32,
    pub capacity: u32,
    pub length:   u32,
}

// note that numbers are stored as little endian
impl Region {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(&self.offset.to_le_bytes());
        buf.extend_from_slice(&self.capacity.to_le_bytes());
        buf.extend_from_slice(&self.length.to_le_bytes());
        buf
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self, Error> {
        if buf.len() != 12 {
            return Err(Error::ParseRegion(buf.len()));
        }

        Ok(Self {
            offset:   u32::from_le_bytes((&buf[0..4]).try_into().unwrap()),
            capacity: u32::from_le_bytes((&buf[4..8]).try_into().unwrap()),
            length:   u32::from_le_bytes((&buf[8..12]).try_into().unwrap()),
        })
    }
}

// ----------------------------------- error -----------------------------------

// we can't use anyhow::Error, because it doesn't implement wasi::core::HostError
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Wasmi(#[from] wasmi::Error),

    #[error("Can't find memory in Wasm exports")]
    MemoryNotFound,

    #[error("Can't find the given function in Wasm exports")]
    FunctionNotFound,

    #[error("Failed to parse Region: expect 12 bytes, found {0}")]
    ParseRegion(usize),

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

    #[error(
        "Failed to write to Wasm memory! offset: {offset}, length: {length}, reason: {reason}"
    )]
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
