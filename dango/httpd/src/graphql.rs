use {
    async_graphql::{Schema, dataloader::DataLoader, extensions},
    indexer_httpd::{
        context::Context,
        graphql::{
            self,
            extensions::metrics::{MetricsExtension, init_graphql_metrics},
            mutation::Mutation,
            telemetry::SentryExtension,
        },
    },
    indexer_sql::dataloaders::{
        block_events::BlockEventsDataLoader, block_transactions::BlockTransactionsDataLoader,
        event_transaction::EventTransactionDataLoader,
        transaction_events::TransactionEventsDataLoader,
        transaction_grug::FileTransactionDataLoader,
        transaction_messages::TransactionMessagesDataLoader,
    },
    query::Query,
    subscription::Subscription,
};

pub mod query;
pub mod subscription;

pub(crate) type AppSchema = Schema<Query, Mutation, Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    init_graphql_metrics();

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

    let event_transaction_loader = DataLoader::new(
        EventTransactionDataLoader {
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
        Query::default(),
        Mutation::default(),
        Subscription::default(),
    )
    .extension(extensions::Logger)
    .extension(extensions::Tracing)
    .extension(SentryExtension)
    .extension(MetricsExtension)
    .data(app_ctx.db.clone())
    .data(app_ctx)
    .data(block_transactions_loader)
    .data(block_events_loader)
    .data(transaction_messages_loader)
    .data(transaction_events_loader)
    .data(file_transaction_loader)
    .data(event_transaction_loader)
    .limit_complexity(200)
    .limit_depth(10)
    .finish()
}
