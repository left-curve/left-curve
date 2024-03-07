use {
    crate::process_query,
    cw_std::{BlockInfo, GenericResult, QueryRequest, QueryResponse, Storage},
    cw_vm::{BackendQuerier, VmResult},
};

pub struct Querier<S> {
    store: S,
    block: BlockInfo,
}

impl<S> Querier<S> {
    pub fn new(store: S, block: BlockInfo) -> Self {
        Self { store, block }
    }
}

impl<S: Storage + Clone + 'static> BackendQuerier for Querier<S> {
    fn query_chain(&self, req: QueryRequest) -> VmResult<GenericResult<QueryResponse>> {
        Ok(process_query(self.store.clone(), &self.block, req).into())
    }
}
