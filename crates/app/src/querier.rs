use {
    crate::{process_query, AppError},
    cw_std::{
        BackendQuerier, BlockInfo, GenericResult, QueryRequest, QueryResponse, StdResult, Storage,
        Vm,
    },
    std::marker::PhantomData,
};

pub struct Querier<S, VM> {
    store: S,
    block: BlockInfo,
    vm: PhantomData<VM>,
}

impl<S, VM> Querier<S, VM> {
    pub fn new(store: S, block: BlockInfo) -> Self {
        Self {
            store,
            block,
            vm: PhantomData,
        }
    }
}

impl<S, VM> BackendQuerier for Querier<S, VM>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    fn query_chain(&self, req: QueryRequest) -> StdResult<GenericResult<QueryResponse>> {
        Ok(process_query::<S, VM>(self.store.clone(), &self.block, req).into())
    }
}
