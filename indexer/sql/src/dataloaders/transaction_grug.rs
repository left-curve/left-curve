use {
    crate::{block_to_index::BlockToIndex, entity, error::IndexerError, indexer_path::IndexerPath},
    anyhow::{anyhow, ensure},
    async_graphql::{dataloader::Loader, *},
    grug_types::{Cache, Tx, TxOutcome},
    std::{collections::HashMap, sync::Arc},
};

type BlockCache<'a> = Cache<'a, u64, BlockToIndex, IndexerError>;

pub struct FileTransactionDataLoader {
    pub indexer: IndexerPath,
}

impl Loader<entity::transactions::Model> for FileTransactionDataLoader {
    type Error = Arc<anyhow::Error>;
    type Value = (Tx, TxOutcome);

    async fn load(
        &self,
        keys: &[entity::transactions::Model],
    ) -> Result<HashMap<entity::transactions::Model, Self::Value>, Self::Error> {
        let mut cache = BlockCache::new(|block_height, _| {
            BlockToIndex::load_from_disk(self.indexer.block_path(*block_height))
        });

        keys.iter()
            .map(|graphql_tx| {
                // Load the block.
                let indexed_block = cache.get_or_fetch(&(graphql_tx.block_height as u64), None)?;

                // Find the transaction in the block.
                let (tx, hash) = indexed_block
                    .block
                    .txs
                    .get(graphql_tx.transaction_idx as usize)
                    .cloned()
                    .ok_or_else(|| anyhow!("transaction not found: {}", graphql_tx.hash))?;

                // Find the transaction outcome in the block.
                let outcome = indexed_block
                    .block_outcome
                    .tx_outcomes
                    .get(graphql_tx.transaction_idx as usize)
                    .cloned()
                    .ok_or_else(|| anyhow!("transaction outcome not found: {}", graphql_tx.hash))?;

                ensure!(
                    hash.to_string() == graphql_tx.hash,
                    "transaction hash mismatch: {} != {}",
                    hash,
                    graphql_tx.hash
                );

                Ok((graphql_tx.clone(), (tx, outcome)))
            })
            .collect::<Result<_, _>>()
            .map_err(Arc::new)
    }
}
