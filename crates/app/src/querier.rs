use {
    crate::{process_query, AppError, Vm},
    grug_types::{BlockInfo, Querier, QueryRequest, QueryResponse, StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct QueryProvider<VM> {
    storage: Box<dyn Storage>,
    block: BlockInfo,
    vm: PhantomData<VM>,
}

impl<VM> QueryProvider<VM> {
    pub fn new(storage: Box<dyn Storage>, block: BlockInfo) -> Self {
        Self {
            storage,
            block,
            vm: PhantomData,
        }
    }
}

impl<VM> Querier for QueryProvider<VM>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    fn query_chain(&self, req: QueryRequest) -> StdResult<QueryResponse> {
        process_query::<VM>(self.storage.clone(), &self.block, req)
            .map_err(|err| StdError::Generic(err.to_string()))
    }
}
