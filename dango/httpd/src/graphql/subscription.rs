use {
    async_graphql::*, indexer_httpd::graphql::subscription::block::BlockSubscription,
    transfer::TransferSubscription,
};

pub mod transfer;

#[derive(MergedSubscription, Default)]
pub struct Subscription(TransferSubscription, BlockSubscription);
