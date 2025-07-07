use crate::indexer_path::IndexerPath;

pub struct Cache {
    pub indexer_path: IndexerPath,
    pub indexing: bool,
    keep_blocks: bool,
}

impl grug_app::Indexer for Cache {
    fn start(&mut self, storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> grug_app::IndexerResult<()> {
        Ok(())
    }

    fn pre_indexing(
        &self,
        block_height: u64,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        Ok(())
    }

    fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        todo!("Store block and block outcome in cache")
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        todo!("Store block and block outcome in ctx")
    }

    fn wait_for_finish(&self) -> grug_app::IndexerResult<()> {
        Ok(())
    }
}
