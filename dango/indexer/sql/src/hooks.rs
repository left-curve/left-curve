use {
    async_trait::async_trait,
    dango_indexer_sql_migration::{Migrator, MigratorTrait},
    indexer_sql::{block_to_index::BlockToIndex, hooks::Hooks as HooksTrait, Context},
};

#[derive(Clone)]
pub struct Hooks;

#[async_trait]
impl HooksTrait for Hooks {
    type Error = crate::error::Error;

    async fn start(&self, context: Context) -> Result<(), Self::Error> {
        Migrator::up(&context.db, None).await?;
        Ok(())
    }

    async fn post_indexing(
        &self,
        _context: Context,
        _block: BlockToIndex,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

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

        let swaps = entity::swaps::Entity::find()
            .all(&indexer.context.db)
            .await?;
        assert_that!(swaps).is_empty();

        Ok(())
    }
}
