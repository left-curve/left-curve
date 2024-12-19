use {
    crate::{process_query, AppError, GasTracker, Vm},
    grug_types::{
        concat, increment_last_byte, trim, BlockInfo, GenericResult, GenericResultExt, Order,
        Querier, Query, QueryResponse, Record, StdError, StdResult, Storage,
    },
};

// ---------------------------------- storage ----------------------------------

/// Provides access to an account's storage to the VM.
///
/// Essentially, this is a prefixed key-value storage. In Grug, the prefix is
/// the single byte `b"w"` (referring to Wasm) followed by the account address.
#[derive(Clone)]
pub struct StorageProvider {
    storage: Box<dyn Storage>,
    namespace: Vec<u8>,
}

impl StorageProvider {
    pub fn new(storage: Box<dyn Storage>, prefixes: &[&[u8]]) -> Self {
        let mut size = 0;
        for prefix in prefixes {
            size += prefix.len();
        }

        let mut namespace = Vec::with_capacity(size);
        for prefix in prefixes {
            namespace.extend_from_slice(prefix);
        }

        Self { storage, namespace }
    }

    pub fn namespace(&self) -> &[u8] {
        &self.namespace
    }
}

impl Storage for StorageProvider {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        let prefixed_key = concat(&self.namespace, key);
        self.storage.read(&prefixed_key)
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let (min, max) = prefixed_range_bounds(&self.namespace, min, max);
        let iter = self
            .storage
            .scan(Some(&min), Some(&max), order)
            .map(|(key, value)| (trim(&self.namespace, &key), value));

        Box::new(iter)
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let (min, max) = prefixed_range_bounds(&self.namespace, min, max);
        let iter = self
            .storage
            .scan_keys(Some(&min), Some(&max), order)
            .map(|key| trim(&self.namespace, &key));

        Box::new(iter)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let (min, max) = prefixed_range_bounds(&self.namespace, min, max);
        self.storage.scan_values(Some(&min), Some(&max), order)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        let prefixed_key = concat(&self.namespace, key);
        self.storage.write(&prefixed_key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        let prefixed_key = concat(&self.namespace, key);
        self.storage.remove(&prefixed_key);
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        let (min, max) = prefixed_range_bounds(&self.namespace, min, max);
        self.storage.remove_range(Some(&min), Some(&max))
    }
}

#[inline]
fn prefixed_range_bounds(
    prefix: &[u8],
    min: Option<&[u8]>,
    max: Option<&[u8]>,
) -> (Vec<u8>, Vec<u8>) {
    let min = match min {
        Some(bytes) => concat(prefix, bytes),
        None => prefix.to_vec(),
    };
    let max = match max {
        Some(bytes) => concat(prefix, bytes),
        None => increment_last_byte(prefix.to_vec()),
    };
    (min, max)
}

// ---------------------------------- querier ----------------------------------

/// Represents the capability to perform queries on the chain.
///
/// ## Notes
///
/// - Compared to `Querier`, which is intended to be used in guest contracts,
///   `QuerierProvider` is intended to be used in the host, and takes an
///   additional parameter `query_depth` to prevent the call stack from getting
///   too deep.
/// - Compared to `StorageProvider`, we use this as a box (`Box<dyn QuerierProvider>`)
///   because performing queries involves calling a VM (while read or write
///   storage doesn't). The VM has to be added as a generic. We need to hide
///   this generic, otherwise we run into infinite recursive types with the
///   hybrid VM. (When using a single VM, this isn't a problem.)
pub trait QuerierProvider {
    fn do_query_chain(&self, req: Query, query_depth: usize) -> GenericResult<QueryResponse>;
}

impl Querier for Box<dyn QuerierProvider> {
    fn query_chain(&self, req: Query) -> StdResult<QueryResponse> {
        // TODO: ignoring query depth for now
        self.do_query_chain(req, 0).map_err(StdError::host)
    }
}

/// Provides querier functionalities to the VM.
pub struct QuerierProviderImpl<VM> {
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
}

impl<VM> QuerierProviderImpl<VM> {
    pub fn new(
        vm: VM,
        storage: Box<dyn Storage>,
        gas_tracker: GasTracker,
        block: BlockInfo,
    ) -> Self {
        Self {
            vm,
            storage,
            gas_tracker,
            block,
        }
    }
}

impl<VM> QuerierProviderImpl<VM>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    pub fn new_boxed(
        vm: VM,
        storage: Box<dyn Storage>,
        gas_tracker: GasTracker,
        block: BlockInfo,
    ) -> Box<dyn QuerierProvider> {
        Box::new(Self::new(vm, storage, gas_tracker, block))
    }
}

impl<VM> QuerierProvider for QuerierProviderImpl<VM>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    fn do_query_chain(&self, req: Query, query_depth: usize) -> GenericResult<QueryResponse> {
        process_query(
            self.vm.clone(),
            self.storage.clone(),
            self.gas_tracker.clone(),
            self.block,
            query_depth,
            req,
        )
        .into_generic_result()
    }
}
