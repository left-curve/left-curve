#[cfg(feature = "metrics")]
use crate::graphql::extensions::metrics::{MetricsExtension, init_graphql_metrics};
use {
    crate::context::Context,
    async_graphql::{
        Schema,
        dataloader::DataLoader,
        extensions::{self as AsyncGraphqlExtensions, OpenTelemetry},
    },
    indexer_sql::dataloaders::{
        block_events::BlockEventsDataLoader, block_transactions::BlockTransactionsDataLoader,
        event_transaction::EventTransactionDataLoader,
        transaction_events::TransactionEventsDataLoader,
        transaction_grug::FileTransactionDataLoader,
        transaction_messages::TransactionMessagesDataLoader,
    },
    telemetry::SentryExtension,
};

pub mod extensions;
pub mod mutation;
pub mod query;
pub mod subscription;
pub mod telemetry;
pub mod types;

pub(crate) type AppSchema = Schema<query::Query, mutation::Mutation, subscription::Subscription>;

pub fn build_schema(app_ctx: Context) -> AppSchema {
    #[cfg(feature = "metrics")]
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

    let mut schema_builder = Schema::build(
        query::Query::default(),
        mutation::Mutation::default(),
        subscription::Subscription::default(),
    )
    .extension(AsyncGraphqlExtensions::Logger)
    .extension(AsyncGraphqlExtensions::Tracing)
    .extension(SentryExtension);

    #[cfg(feature = "metrics")]
    {
        schema_builder = schema_builder.extension(MetricsExtension);
    }

    schema_builder
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
