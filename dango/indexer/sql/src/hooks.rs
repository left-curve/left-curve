use {
    async_trait::async_trait,
    dango_indexer_sql_migration::{Migrator, MigratorTrait},
    grug_app::QuerierProvider,
    indexer_sql::{Context, block_to_index::BlockToIndex, hooks::Hooks as HooksTrait},
};
#[cfg(feature = "metrics")]
use {
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

mod accounts;
mod transfers;

#[derive(Clone)]
pub struct Hooks;

#[async_trait]
impl HooksTrait for Hooks {
    type Error = crate::error::Error;

    async fn start(&self, context: Context) -> Result<(), Self::Error> {
        Migrator::up(&context.db, None).await?;

        #[cfg(feature = "metrics")]
        {
            transfers::init_metrics();
            accounts::init_metrics();
            init_metrics();
        }

        Ok(())
    }

    async fn post_indexing(
        &self,
        context: Context,
        block: BlockToIndex,
        querier: &dyn QuerierProvider,
    ) -> Result<(), Self::Error> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        self.save_transfers(&context, &block).await?;
        self.save_accounts(&context, &block, querier).await?;

        #[cfg(feature = "metrics")]
        histogram!(
            "indexer.dango.hooks.duration",
            "block_height" => block.block.info.height.to_string()
        )
        .record(start.elapsed().as_secs_f64());

        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    describe_histogram!("indexer.dango.hooks.duration", "Hook duration in seconds");
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*, crate::entity, assertor::*, grug_app::Indexer, grug_types::MockStorage,
        indexer_sql::non_blocking_indexer::IndexerBuilder, sea_orm::EntityTrait,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn build_with_hooks() -> anyhow::Result<()> {
        let mut indexer = IndexerBuilder::default()
            .with_memory_database()
            .with_tmpdir()
            .with_hooks(Hooks)
            .build()?;

        let storage = MockStorage::new();

        assert!(!indexer.indexing);
        indexer.start(&storage).expect("Can't start Indexer");
        assert!(indexer.indexing);

        indexer.shutdown().expect("Can't shutdown Indexer");
        assert!(!indexer.indexing);

        let transfers = entity::transfers::Entity::find()
            .all(&indexer.context.db)
            .await?;
        assert_that!(transfers).is_empty();

        Ok(())
    }
}
