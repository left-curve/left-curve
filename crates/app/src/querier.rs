use {
    crate::{process_query, AppError, SharedGasTracker, Vm},
    grug_types::{BlockInfo, Querier, QueryRequest, QueryResponse, StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct QueryProvider<VM> {
    storage: Box<dyn Storage>,
    block: BlockInfo,
    vm: PhantomData<VM>,
    gas_tracker: SharedGasTracker,
}

impl<VM> QueryProvider<VM> {
    pub fn new(storage: Box<dyn Storage>, block: BlockInfo, gas_tracker: SharedGasTracker) -> Self {
        Self {
            storage,
            block,
            vm: PhantomData,
            gas_tracker,
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
            req,
        )
        .map_err(|err| StdError::Generic(err.to_string()))
    }
}
