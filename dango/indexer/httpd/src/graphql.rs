#[cfg(feature = "metrics")]
use indexer_httpd::graphql::extensions::metrics::{MetricsExtension, init_graphql_metrics};
use {
    async_graphql::{Schema, dataloader::DataLoader, extensions},
    indexer_httpd::graphql::{mutation::Mutation, telemetry::SentryExtension},
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
            indexer: dango_httpd_context
                .indexer_httpd_context
                .indexer_path
                .clone(),
        },
        tokio::spawn,
    );

    #[allow(unused_mut)]
    let mut schema_builder = Schema::build(
        Query::default(),
        Mutation::default(),
        Subscription::default(),
    )
    .extension(extensions::Logger)
        // .extension(extensions::Tracing)
    .extension(SentryExtension);

    #[cfg(feature = "metrics")]
    {
        schema_builder = schema_builder.extension(MetricsExtension);
    }

    schema_builder
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
        .limit_complexity(200)
        .limit_depth(10)
        .finish()
}
