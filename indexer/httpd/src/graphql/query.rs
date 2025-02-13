use {
    async_graphql::MergedObject, block::BlockQuery, event::EventQuery, message::MessageQuery,
    tendermint::TendermintQuery, transaction::TransactionQuery,
};

pub mod block;
pub mod event;
pub mod message;
pub mod tendermint;
pub mod transaction;

#[derive(MergedObject, Default)]
pub struct Query(
    BlockQuery,
    TransactionQuery,
    MessageQuery,
    EventQuery,
    TendermintQuery,
);
