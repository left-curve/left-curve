use {
    crate::{process_query, AppError, Vm},
    cw_types::{BlockInfo, GenericResult, QueryRequest, QueryResponse, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct Querier<VM> {
    store: Box<dyn Storage>,
    block: BlockInfo,
    vm: PhantomData<VM>,
}

impl<VM> Querier<VM>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    pub fn new(store: Box<dyn Storage>, block: BlockInfo) -> Self {
        Self {
            store,
            block,
            vm: PhantomData,
        }
    }

    pub fn query_chain(&self, req: QueryRequest) -> StdResult<GenericResult<QueryResponse>> {
        Ok(process_query::<_, VM>(self.store.clone(), &self.block, req).into())
    }
}
