use {
    crate::Region,
    anyhow::{anyhow, bail, Context},
    cw_sdk::{Order, Storage},
    data_encoding::BASE64,
    std::{cell::OnceCell, collections::HashMap},
    wasmi::{Caller, Extern, Instance, Memory, Store, TypedFunc, WasmParams, WasmResults},
};

/// The state of the Wasm host. It includes a key-value store (which must
/// implement the cw_sdk::Storage trait) and a registry of iterators.
pub struct HostState<S> {
    // we typically name this "store", but to avoid confusion with wasmi::Store,
    // we call this "kv" here.
    pub(crate) kv: S,

    // iterators, indexed by iterator_id
    pub(crate) iterators: HashMap<u32, Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)>>>,

    // the next time an iterator is created, the ID it will get.
    // incremented by 1 upon each iterator creation.
    pub(crate) next_iterator_id: u32,
}

impl<S> HostState<S> {
    pub fn new(kv: S) -> Self {
        Self {
            kv,
            iterators: HashMap::new(),
            next_iterator_id: 0, // should we start from 0 or 1?
        }
    }
}

impl<S> HostState<S>
where
    S: Storage,
{
    /// Create a new iterator over the KV store, return the iterator_id.
    pub fn create_iterator(&mut self, min: Option<&[u8]>, max: Option<&[u8]>, order: Order) -> u32 {
        let iterator_id = self.next_iterator_id;
        // TODO: do we need to handle the case where this overflows?
        self.next_iterator_id += 1;

        let iterator = self.kv.scan(min, max, order);
        self.iterators.insert(iterator_id, iterator);

        iterator_id
    }

    /// Get a mutable reference to the iterator specified by the given ID.
    ///
    /// NOTE: the iterator id must exist.
    pub fn get_iterator_mut(
        &mut self,
        iterator_id: u32,
    ) -> &mut Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)>> {
        let maybe_iter = self.iterators.get_mut(&iterator_id);
        debug_assert!(maybe_iter.is_some(), "Iterator with id {iterator_id} doesn't exist");
        maybe_iter.unwrap()
    }

    /// Delete an iterator from the host state. Called by the host once
    /// iteration has reached end.
    ///
    /// NOTE: the iterator id must exist.
    pub fn drop_iterator(&mut self, iterator_id: u32) {
        let maybe_iter = self.iterators.remove(&iterator_id);
        debug_assert!(maybe_iter.is_some(), "Iterator with id {iterator_id} doesn't exist");
    }
}

pub struct Host<'a, S> {
    caller:     Caller<'a, HostState<S>>,
    memory:     OnceCell<Memory>,
    alloc_fn:   OnceCell<TypedFunc<u32, u32>>,
    dealloc_fn: OnceCell<TypedFunc<u32, ()>>,
}

impl<'a, S> From<Caller<'a, HostState<S>>> for Host<'a, S> {
    fn from(caller: Caller<'a, HostState<S>>) -> Self {
        Self {
            caller,
            memory:     OnceCell::new(),
            alloc_fn:   OnceCell::new(),
            dealloc_fn: OnceCell::new(),
        }
    }
}

impl<'a, S> Host<'a, S> {
    pub fn new(instance: &Instance, store: &'a mut Store<HostState<S>>) -> Self {
        Self {
            caller:     Caller::new(store, Some(instance)),
            memory:     OnceCell::new(),
            alloc_fn:   OnceCell::new(),
            dealloc_fn: OnceCell::new(),
        }
    }

    /// Get an immutable reference to the host state.
    pub fn data(&self) -> &HostState<S> {
        self.caller.data()
    }

    /// Get a mutable reference to the host state.
    pub fn data_mut(&mut self) -> &mut HostState<S> {
        self.caller.data_mut()
    }

    /// Call a function on the Wasm module.
    pub fn call<P, R>(&mut self, name: &str, params: P) -> anyhow::Result<R>
    where
        P: WasmParams,
        R: WasmResults,
    {
        self.get_typed_func(name)?
            .call(&mut self.caller, params)
            .map_err(Into::into)
    }

    /// Reserve a region in Wasm memory and write the given data into it.
    pub fn write_to_memory(&mut self, data: &[u8]) -> anyhow::Result<u32> {
        let region_ptr = self.alloc_fn().call(&mut self.caller, data.len() as u32)?;
        self.write_region(region_ptr, data)?;
        Ok(region_ptr)
    }

    /// Read data from a region in Wasm memory.
    pub fn read_from_memory(&self, region_ptr: u32) -> anyhow::Result<Vec<u8>> {
        let buf = self.read_memory(region_ptr as usize, Region::SIZE)?;
        let region = unsafe { Region::from_raw(&buf) };

        self.read_memory(region.offset as usize, region.length as usize)
    }

    /// Read data from a region then deallocate it. This is used almost
    /// exclusively for reading the response at the very end of the call.
    /// For all other use cases, Host::read_from_memory probably should be used.
    pub fn read_then_wipe(&mut self, region_ptr: u32) -> anyhow::Result<Vec<u8>> {
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
