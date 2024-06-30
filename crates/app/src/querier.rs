use {
    crate::{process_query, AppError, SharedCacheModules, SharedGasTracker, Vm},
    grug_types::{BlockInfo, Querier, QueryRequest, QueryResponse, StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct QueryProvider<VM: Vm> {
    storage: Box<dyn Storage>,
    block: BlockInfo,
    vm: PhantomData<VM>,
    gas_tracker: SharedGasTracker,
    cache_module: SharedCacheModules<VM>,
}

impl<VM> QueryProvider<VM>
where
    VM: Vm,
{
    pub fn new(
        storage: Box<dyn Storage>,
        block: BlockInfo,
        gas_tracker: SharedGasTracker,
        cache_module: SharedCacheModules<VM>,
    ) -> Self {
        Self {
            storage,
            block,
            vm: PhantomData,
            gas_tracker,
            cache_module,
        }
    }
}

impl<VM> Querier for QueryProvider<VM>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    fn query_chain(&self, req: QueryRequest) -> StdResult<QueryResponse> {
        process_query::<VM>(
            self.storage.clone(),
            self.block.clone(),
            self.gas_tracker.clone(),
            self.cache_module.clone(),
            req,
        )
        .map_err(|err| StdError::Generic(err.to_string()))
    }
}
