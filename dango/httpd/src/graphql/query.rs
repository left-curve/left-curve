use {async_graphql::MergedObject, transfer::TransferQuery};

pub mod transfer;

#[derive(MergedObject, Default)]
pub struct Query(TransferQuery);
