use {
    crate::context::Context,
    async_graphql::{Schema, dataloader::DataLoader, extensions},
    indexer_sql::dataloaders::{
        block_events::BlockEventsDataLoader, block_transactions::BlockTransactionsDataLoader,
        transaction_events::TransactionEventsDataLoader,
        transaction_grug::FileTransactionDataLoader,
        transaction_messages::TransactionMessagesDataLoader,
    },
    telemetry::SentryExtension,
};

pub mod mutation;
pub mod query;
pub mod subscription;
pub mod telemetry;
pub mod types;

pub(crate) type AppSchema = Schema<query::Query, mutation::Mutation, subscription::Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    let block_transactions_loader = DataLoader::new(
        BlockTransactionsDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    let block_events_loader = DataLoader::new(
        BlockEventsDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    let transaction_messages_loader = DataLoader::new(
        TransactionMessagesDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    let transaction_events_loader = DataLoader::new(
        TransactionEventsDataLoader {
            db: app_ctx.db.clone(),
        },
        tokio::spawn,
    );

    let file_transaction_loader = DataLoader::new(
        FileTransactionDataLoader {
            indexer: app_ctx.indexer_path.clone(),
        },
        tokio::spawn,
    );

    Schema::build(
        query::Query::default(),
        mutation::Mutation::default(),
        subscription::Subscription::default(),
    )
    .extension(extensions::Logger)
    .extension(SentryExtension)
    .data(app_ctx.db.clone())
    .data(app_ctx)
    .data(block_transactions_loader)
    .data(block_events_loader)
    .data(transaction_messages_loader)
    .data(transaction_events_loader)
    .data(file_transaction_loader)
    .finish()
}
