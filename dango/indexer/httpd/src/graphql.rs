#[cfg(feature = "metrics")]
use crate::graphql::extensions::metrics::{MetricsExtension, init_graphql_metrics};
#[cfg(feature = "tracing")]
use async_graphql::extensions as AsyncGraphqlExtensions;
use {
    crate::graphql::{
        mutation::IndexerMutation, query::Query, subscription::Subscription,
        telemetry::SentryExtension,
    },
    async_graphql::{Schema, dataloader::DataLoader},
    indexer_sql::dataloaders::{
        block_events::BlockEventsDataLoader, block_transactions::BlockTransactionsDataLoader,
        event_transaction::EventTransactionDataLoader,
        transaction_events::TransactionEventsDataLoader,
        transaction_grug::FileTransactionDataLoader,
        transaction_messages::TransactionMessagesDataLoader,
    },
};

pub mod extensions;
pub mod mutation;
pub mod query;
pub mod subscription;
pub mod telemetry;
pub mod types;

pub(crate) type AppSchema = Schema<Query, IndexerMutation, Subscription>;

pub fn build_schema(dango_httpd_context: crate::context::Context) -> AppSchema {
    #[cfg(feature = "metrics")]
    init_graphql_metrics();

    let block_transactions_loader = DataLoader::new(
        BlockTransactionsDataLoader {
            db: dango_httpd_context.db.clone(),
        },
        tokio::spawn,
    );

    let block_events_loader = DataLoader::new(
        BlockEventsDataLoader {
            db: dango_httpd_context.db.clone(),
        },
        tokio::spawn,
    );

    let event_transaction_loader = DataLoader::new(
        EventTransactionDataLoader {
            db: dango_httpd_context.db.clone(),
        },
        tokio::spawn,
    );

    let transaction_messages_loader = DataLoader::new(
        TransactionMessagesDataLoader {
            db: dango_httpd_context.db.clone(),
        },
        tokio::spawn,
    );

    let transaction_events_loader = DataLoader::new(
        TransactionEventsDataLoader {
            db: dango_httpd_context.db.clone(),
        },
        tokio::spawn,
    );

    let file_transaction_loader = DataLoader::new(
        FileTransactionDataLoader {
            indexer_path: dango_httpd_context
                .indexer_httpd_context
                .indexer_cache_context
                .indexer_path
                .clone(),
        },
        tokio::spawn,
    );

    let indexer_path = dango_httpd_context
        .indexer_httpd_context
        .indexer_cache_context
        .indexer_path
        .clone();

    #[allow(unused_mut)]
    let mut schema_builder = Schema::build(
        Query::default(),
        IndexerMutation::default(),
        Subscription::default(),
    )
    .extension(SentryExtension);

    #[cfg(feature = "metrics")]
    {
        schema_builder = schema_builder.extension(MetricsExtension);
    }

    #[cfg(feature = "tracing")]
    {
        schema_builder = schema_builder
            .extension(AsyncGraphqlExtensions::Tracing)
            .extension(AsyncGraphqlExtensions::Logger);
    }

    schema_builder
        .data(dango_httpd_context.indexer_clickhouse_context.clone())
        .data(dango_httpd_context.indexer_httpd_context.base.clone())
        .data(dango_httpd_context.indexer_httpd_context.clone())
        .data(dango_httpd_context.db.clone())
        .data(dango_httpd_context)
        .data(block_transactions_loader)
        .data(block_events_loader)
        .data(transaction_messages_loader)
        .data(transaction_events_loader)
        .data(file_transaction_loader)
        .data(event_transaction_loader)
        .data(indexer_path)
        .limit_complexity(300)
        .limit_depth(20)
        .finish()
}
