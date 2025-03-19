use {
    async_graphql::{dataloader::DataLoader, extensions, Schema},
    indexer_httpd::{
        context::Context,
        graphql::{dataloader, mutation::Mutation},
    },
    query::Query,
    subscription::Subscription,
};

pub mod query;
pub mod subscription;
pub mod types;

pub(crate) type AppSchema = Schema<Query, Mutation, Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    let block_transactions_loader = DataLoader::new(
        dataloader::transaction::TransactionDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    Schema::build(
        Query::default(),
        Mutation::default(),
        Subscription::default(),
    )
    .extension(extensions::Logger)
    .data(app_ctx)
    .data(block_transactions_loader)
    .finish()
}
