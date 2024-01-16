use {
    crate::{BackendQuerier, VmResult},
    cw_std::{GenericResult, QueryRequest, QueryResponse},
};

pub struct MockBackendQuerier;

impl BackendQuerier for MockBackendQuerier {
    fn query_chain(&self, _req: QueryRequest) -> VmResult<GenericResult<QueryResponse>> {
        todo!("MockBackendQuerier isn't implemented")
    }
}
