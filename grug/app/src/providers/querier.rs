use {
    crate::{AppError, GasTracker, Vm, process_query},
    grug_types::{
        BlockInfo, GenericResult, GenericResultExt, Querier, Query, QueryResponse, StdError,
        StdResult, Storage,
    },
};

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
