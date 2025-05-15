use {
    async_graphql::{Schema, dataloader::DataLoader, extensions},
    indexer_httpd::{
        context::Context,
        graphql::{dataloader, mutation::Mutation, telemetry::SentryExtension},
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
        dataloader::block_transactions::BlockTransactionsDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    let block_events_loader = DataLoader::new(
        dataloader::block_events::BlockEventsDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    let transaction_messages_loader = DataLoader::new(
        dataloader::transaction_messages::TransactionMessagesDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    let transaction_events_loader = DataLoader::new(
        dataloader::transaction_events::TransactionEventsDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    let file_transaction_loader: DataLoader<
        dataloader::file_transaction::FileTranscationDataLoader,
    > = DataLoader::new(
        dataloader::file_transaction::FileTranscationDataLoader {
            indexer: app_ctx.indexer_path.clone(),
        },
        tokio::spawn,
    );

    Schema::build(
        Query::default(),
        Mutation::default(),
        Subscription::default(),
    )
    .extension(extensions::Logger)
    .extension(SentryExtension)
    .data(app_ctx)
    .data(block_transactions_loader)
    .data(block_events_loader)
    .data(transaction_messages_loader)
    .data(transaction_events_loader)
    .data(file_transaction_loader)
    .finish()
}
