use {
    crate::{IndexerError, block_to_index::BlockToIndex, entity, indexer_path::IndexerPath},
    async_graphql::dataloader::Loader,
    grug_types::{Cache, JsonSerExt},
    std::{collections::HashMap, sync::Arc},
};

type BlockCache<'a> = Cache<'a, u64, BlockToIndex, IndexerError>;

pub struct BlockCronsOutcomesDataLoader {
    pub indexer: IndexerPath,
}

impl Loader<entity::blocks::Model> for BlockCronsOutcomesDataLoader {
    type Error = Arc<anyhow::Error>;
    type Value = Vec<String>;

    async fn load(
        &self,
        keys: &[entity::blocks::Model],
    ) -> Result<HashMap<entity::blocks::Model, Self::Value>, Self::Error> {
        let mut cache = BlockCache::new(|block_height, _| {
            BlockToIndex::load_from_disk(self.indexer.block_path(*block_height))
        });

        keys.iter()
            .map(|block| {
                let indexed_block = cache.get_or_fetch(&(block.block_height as u64), None)?;

                let crons_outcomes = indexed_block
                    .block_outcome
                    .cron_outcomes
                    .iter()
                    .map(|cron| cron.to_json_string())
                    .collect::<Result<Vec<String>, _>>()?;

                Ok((block.clone(), crons_outcomes))
            })
            .collect::<Result<_, _>>()
            .map_err(Arc::new)
    }
}
