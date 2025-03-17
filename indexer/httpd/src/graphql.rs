use {
    crate::context::Context,
    async_graphql::{dataloader::DataLoader, extensions, Schema},
};

pub mod dataloader;
pub mod mutation;
pub mod query;
pub mod subscription;
pub mod types;

pub(crate) type AppSchema = Schema<query::Query, mutation::Mutation, subscription::Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    let block_transactions_loader = DataLoader::new(
        dataloader::transaction::TransactionDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    Schema::build(
        query::Query::default(),
        mutation::Mutation::default(),
        subscription::Subscription::default(),
    )
    .extension(extensions::Logger)
    .data(app_ctx)
    .data(block_transactions_loader)
    .finish()
}
