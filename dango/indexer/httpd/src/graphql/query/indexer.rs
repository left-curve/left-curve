use {
    super::{
        block::BlockQuery, event::EventQuery, grug::GrugQuery, message::MessageQuery,
        transaction::TransactionQuery,
    },
    async_graphql::MergedObject,
};

#[derive(MergedObject, Default)]
pub struct IndexerQuery(
    BlockQuery,
    TransactionQuery,
    MessageQuery,
    EventQuery,
    GrugQuery,
);
