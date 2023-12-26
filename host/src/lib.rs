use {
    anyhow::Context,
    std::{fs::File, mem::size_of, path::Path},
    wasmi::{
        core::{HostError, Trap},
        errors::MemoryError,
        AsContext, AsContextMut, Caller, Engine, Instance, IntoFunc, Linker, Module, Store,
        TypedFunc, WasmParams, WasmResults,
    },
};

/// Wraps around the wasmi Instance and a Store, providing some convenience
/// methods.
pub struct Host<HostState> {
    instance:  wasmi::Instance,
    store:     Store<HostState>,
    memory:    Memory,
    allocator: Allocator,
}

impl<HostState> Host<HostState> {
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


#[derive(Default)]
pub struct HostBuilder<HostState> {
    engine: Engine,
    module: Option<Module>,
    store:  Option<Store<HostState>>,
    linker: Option<Linker<HostState>>,
}

impl<HostState> HostBuilder<HostState> {
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

    pub fn finalize(mut self) -> anyhow::Result<Host<HostState>> {
        let module = self.take_module()?;
        let mut store = self.take_store()?;
        let linker = self.take_linker()?;
        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

        Host::new(instance, store)
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

/// Parallel to sdk::Region
#[derive(Debug)]
pub struct Region {
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

/// A wrapper over wasmi::Memory, providing some convenience methods.
#[derive(Debug, Clone, Copy)]
pub struct Memory {
    inner: wasmi::Memory,
}

impl<T> TryFrom<(&Instance, &Store<T>)> for Memory {
    type Error = anyhow::Error;

    fn try_from((instance, store): (&Instance, &Store<T>)) -> anyhow::Result<Self> {
        instance
            .get_memory(store, "memory")
            .map(Self::new)
            .context("Failed to get memory from instance")
    }
}

impl<'a, T> TryFrom<&Caller<'a, T>> for Memory {
    type Error = Trap;

    fn try_from(caller: &Caller<'a, T>) -> Result<Self, Trap> {
        caller
            .get_export("memory")
            .ok_or(Error::ExportNotFound)?
            .into_memory()
            .map(Self::new)
            .ok_or(Error::ExportIsNotMemory.into())
    }
}

impl Memory {
    pub fn new(inner: wasmi::Memory) -> Self {
        Self { inner }
    }

    pub fn read_region(&self, ctx: impl AsContext, region_ptr: u32) -> Result<Vec<u8>, Trap> {
        let buf = self.read(&ctx, region_ptr as usize, size_of::<Region>())?;
        let region = Region::deserialize(&buf)?;

        self.read(ctx, region.offset as usize, region.length as usize)
    }

    pub fn write_region(
        &self,
        mut ctx: impl AsContextMut,
        region_ptr: u32,
        data: &[u8],
    ) -> Result<(), Trap> {
        let buf = self.read(&ctx, region_ptr as usize, size_of::<Region>())?;
        let mut region = Region::deserialize(&buf)?;
        // don't forget to update the Region length
        region.length = data.len() as u32;

        if region.length > region.capacity {
            return Err(Error::InsufficientRegion {
                capacity: region.capacity,
                length:   region.length,
            }
            .into());
        }

        self.write(&mut ctx, region.offset as usize, data)?;
        self.write(&mut ctx, region_ptr as usize, &region.serialize())
    }

    fn read(&self, ctx: impl AsContext, offset: usize, length: usize) -> Result<Vec<u8>, Trap> {
        let mut buf = vec![0x8; length];
        self.inner
            .read(ctx, offset, &mut buf)
            .map(|_| buf)
            .map_err(|reason| Error::ReadMemory {
                offset,
                length,
                reason,
            }
            .into())
    }

    fn write(&self, ctx: impl AsContextMut, offset: usize, data: &[u8]) -> Result<(), Trap> {
        self.inner
            .write(ctx, offset, data)
            .map_err(|reason| Error::WriteMemory {
                offset,
                length: data.len(),
                reason,
            }
            .into())
    }
}

pub struct Allocator {
    alloc_fn:   TypedFunc<u32, u32>,
    dealloc_fn: TypedFunc<u32, ()>,
}

impl<T> TryFrom<(&Instance, &Store<T>)> for Allocator {
    type Error = anyhow::Error;

    fn try_from((instance, store): (&Instance, &Store<T>)) -> anyhow::Result<Self> {
        let alloc_fn = instance.get_typed_func(store, "allocate")?;
        let dealloc_fn = instance.get_typed_func(store, "deallocate")?;
        Ok(Self { alloc_fn, dealloc_fn })
    }
}

impl<'a, T> TryFrom<&Caller<'a, T>> for Allocator {
    type Error = Trap;

    fn try_from(caller: &Caller<'a, T>) -> Result<Self, Trap> {
        let alloc_fn = caller
            .get_export("allocate")
            .ok_or(Error::ExportNotFound)?
            .into_func()
            .ok_or(Error::ExportIsNotFunc)?
            .typed(&caller)
            .map_err(Error::from)?;
        let dealloc_fn = caller
            .get_export("deallocate")
            .ok_or(Error::ExportNotFound)?
            .into_func()
            .ok_or(Error::ExportIsNotFunc)?
            .typed(&caller)
            .map_err(Error::from)?;
        Ok(Self { alloc_fn, dealloc_fn })
    }
}

impl Allocator {
    pub fn allocate(&self, ctx: impl AsContextMut, capacity: usize) -> Result<u32, Trap> {
        self.alloc_fn.call(ctx, capacity as u32)
    }

    pub fn deallocate(&self, ctx: impl AsContextMut, region_ptr: u32) -> Result<(), Trap> {
        self.dealloc_fn.call(ctx, region_ptr)
    }
}

// we can't use anyhow::Error, because it doesn't implement wasi::core::HostError
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Wasmi(#[from] wasmi::Error),

    #[error("Can't find the desired Wasm export")]
    ExportNotFound,

    #[error("Export is not a function")]
    ExportIsNotFunc,

    #[error("Export is not a memory")]
    ExportIsNotMemory,

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

// important
impl HostError for Error {}
