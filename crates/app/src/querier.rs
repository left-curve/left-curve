use {
    crate::{process_query, AppError, Vm},
    cw_types::{BlockInfo, Querier, QueryRequest, QueryResponse, StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct QueryProvider<VM> {
    store: Box<dyn Storage>,
    block: BlockInfo,
    vm: PhantomData<VM>,
}

impl<VM> QueryProvider<VM> {
    pub fn new(store: Box<dyn Storage>, block: BlockInfo) -> Self {
        Self {
            store,
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
        process_query::<VM>(self.store.clone(), &self.block, req)
            .map_err(|err| StdError::Generic(err.to_string()))
    }
}
