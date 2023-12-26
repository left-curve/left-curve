use {
    anyhow::Context,
    std::{cell::OnceCell, fs::File, mem::size_of, path::Path},
    wasmi::{
        core::{HostError, Trap},
        errors::MemoryError,
        AsContext, AsContextMut, Caller, Engine, Instance, IntoFunc, Linker, Module, Store,
        TypedFunc, WasmParams, WasmResults, Memory, StoreContext, StoreContextMut
    },
};

// ----------------------------------- inner -----------------------------------

enum HostInner<'a, HostState> {
    Owned {
        instance: Instance,
        store:    Store<HostState>,
    },
    Ref(Caller<'a, HostState>),
}

impl<'a, HostState> AsContext for HostInner<'a, HostState> {
    type UserState = HostState;

    fn as_context(&self) -> StoreContext<HostState> {
        match self {
            HostInner::Owned { store, .. } => store.as_context(),
            HostInner::Ref(caller) => caller.as_context(),
        }
    }
}

impl<'a, HostState> AsContextMut for HostInner<'a, HostState> {
    fn as_context_mut(&mut self) -> StoreContextMut<HostState> {
        match self {
            HostInner::Owned { store, .. } => store.as_context_mut(),
            HostInner::Ref(caller) => caller.as_context_mut(),
        }
    }
}

impl<'a, HostState> HostInner<'a, HostState> {
    fn call<P, R>(&mut self, name: &str, params: P) -> Result<R, Trap>
    where
        P: WasmParams,
        R: WasmResults,
    {
        let HostInner::Owned { instance, store } = self else {
            unimplemented!();
        };

        instance
            .get_typed_func(&store, name)
            .map_err(Error::from)?
            .call(store, params)
    }

    fn recycle(self) -> HostState {
        let HostInner::Owned { store, .. } = self else {
            unimplemented!();
        };

        store.into_data()
    }

    fn data(&self) -> &HostState {
        match self {
            HostInner::Owned { store, .. } => store.data(),
            HostInner::Ref(caller) => caller.data(),
        }
    }

    fn data_mut(&mut self) -> &mut HostState {
        match self {
            HostInner::Owned { store, .. } => store.data_mut(),
            HostInner::Ref(caller) => caller.data_mut(),
        }
    }

    fn memory(&self) -> Result<Memory, Error> {
        match self {
            HostInner::Owned { instance, store } => instance
                .get_memory(&store, "memory")
                .ok_or(Error::FailedToObtainMemory),
            HostInner::Ref(caller) => caller
                .get_export("memory")
                .ok_or(Error::ExportNotFound)?
                .into_memory()
                .ok_or(Error::ExportIsNotMemory),
        }
    }

    fn alloc_fn(&self) -> Result<TypedFunc<u32, u32>, Error> {
        match self {
            HostInner::Owned { instance, store } => instance
                .get_typed_func(&store, "allocate"),
            HostInner::Ref(caller) => caller
                .get_export("allocate")
                .ok_or(Error::ExportNotFound)?
                .into_func()
                .ok_or(Error::ExportIsNotFunc)?
                .typed(&caller)
        }
        .map_err(Into::into)
    }

    fn dealloc_fn(&self) -> Result<TypedFunc<u32, ()>, Error> {
        match self {
            HostInner::Owned { instance, store } => instance
                .get_typed_func(&store, "deallocate"),
            HostInner::Ref(caller) => caller
                .get_export("deallocate")
                .ok_or(Error::ExportNotFound)?
                .into_func()
                .ok_or(Error::ExportIsNotFunc)?
                .typed(&caller)
        }
        .map_err(Into::into)
    }
}

// ----------------------------------- host ------------------------------------

pub struct Host<'a, HostState> {
    inner:      HostInner<'a, HostState>,
    memory:     OnceCell<Memory>,
    alloc_fn:   OnceCell<TypedFunc<u32, u32>>,
    dealloc_fn: OnceCell<TypedFunc<u32, ()>>,
}

impl<HostState> Host<'_, HostState> {
    pub fn build_owned(
        instance: Instance,
        store:    Store<HostState>,
    ) -> Result<Self, Error> {
        Ok(Self {
            inner:      HostInner::Owned { instance, store },
            memory:     OnceCell::new(),
            alloc_fn:   OnceCell::new(),
            dealloc_fn: OnceCell::new(),
        })
    }
}

impl<'a, HostState> Host<'a, HostState> {
    pub fn build_ref(caller: Caller<'a, HostState>) -> Result<Self, Error> {
        Ok(Self {
            inner:      HostInner::Ref(caller),
            memory:     OnceCell::new(),
            alloc_fn:   OnceCell::new(),
            dealloc_fn: OnceCell::new(),
        })
    }
}

impl<HostState> Host<'_, HostState> {
    pub fn call<P, R>(&mut self, name: &str, params: P) -> Result<R, Trap>
    where
        P: WasmParams,
        R: WasmResults,
    {
        self.inner.call(name, params)
    }

    pub fn recycle(self) -> HostState {
        self.inner.recycle()
    }

    pub fn data(&self) -> &HostState {
        self.inner.data()
    }

    pub fn data_mut(&mut self) -> &mut HostState {
        self.inner.data_mut()
    }

    pub fn release_buffer(&mut self, data: Vec<u8>) -> Result<u32, Trap> {
        let region_ptr = self.alloc_fn().call(&mut self.inner, data.capacity() as u32)?;
        self.write_region(region_ptr, &data)?;
        Ok(region_ptr)
    }

    pub fn consume_region(&mut self, region_ptr: u32) -> Result<Vec<u8>, Trap> {
        let data = self.read_region(region_ptr)?;
        self.dealloc_fn().call(&mut self.inner, region_ptr)?;
        Ok(data)
    }

    pub fn read_region(&self, region_ptr: u32) -> Result<Vec<u8>, Trap> {
        let buf = self.read_memory(region_ptr as usize, size_of::<Region>())?;
        let region = Region::deserialize(&buf)?;

        self.read_memory(region.offset as usize, region.length as usize)
    }

    fn write_region(
        &mut self,
        region_ptr: u32,
        data: &[u8],
    ) -> Result<(), Trap> {
        let buf = self.read_memory(region_ptr as usize, size_of::<Region>())?;
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

        self.write_memory(region.offset as usize, data)?;
        self.write_memory(region_ptr as usize, &region.serialize())
    }

    fn read_memory(&self, offset: usize, length: usize) -> Result<Vec<u8>, Trap> {
        let mut buf = vec![0x8; length];
        self.memory()
            .read(&self.inner, offset, &mut buf)
            .map(|_| buf)
            .map_err(|reason| Error::ReadMemory {
                offset,
                length,
                reason,
            }
            .into())
    }

    fn write_memory(&mut self, offset: usize, data: &[u8]) -> Result<(), Trap> {
        self.memory()
            .write(&mut self.inner, offset, data)
            .map_err(|reason| Error::WriteMemory {
                offset,
                length: data.len(),
                reason,
            }
            .into())
    }

    fn memory(&self) -> Memory {
        // memory should implement Copy but it doesn't for some reason
        self.memory.get_or_init(|| self.inner.memory().unwrap()).clone()
    }

    fn alloc_fn(&self) -> TypedFunc<u32, u32> {
        self.alloc_fn.get_or_init(|| self.inner.alloc_fn().unwrap()).clone()
    }

    fn dealloc_fn(&self) -> TypedFunc<u32, ()> {
        self.dealloc_fn.get_or_init(|| self.inner.dealloc_fn().unwrap()).clone()
    }
}

// ---------------------------------- builder ----------------------------------

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

    pub fn finalize(mut self) -> anyhow::Result<Host<'static, HostState>> {
        let module = self.take_module()?;
        let mut store = self.take_store()?;
        let linker = self.take_linker()?;
        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

        Host::build_owned(instance, store).map_err(Into::into)
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

    #[error("Failed to get memory from instance")]
    FailedToObtainMemory,

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
