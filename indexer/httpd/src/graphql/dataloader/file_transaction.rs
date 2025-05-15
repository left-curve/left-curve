use {
    crate::graphql::types::transaction::Transaction,
    anyhow::{anyhow, ensure},
    async_graphql::dataloader::Loader,
    grug_types::{Tx, TxOutcome},
    indexer_sql::{block_to_index::BlockToIndex, error::IndexerError, indexer_path::IndexerPath},
    std::{
        collections::{HashMap, hash_map::Entry},
        sync::Arc,
    },
};

pub struct BlockCache<'a> {
    pub blocks: HashMap<u64, BlockToIndex>,
    pub indexer: &'a IndexerPath,
}

impl BlockCache<'_> {
    pub fn new(indexer: &IndexerPath) -> BlockCache {
        BlockCache {
            indexer,
            blocks: HashMap::new(),
        }
    }

    pub fn get_or_load_from_disk(
        &mut self,
        block_height: u64,
    ) -> Result<&BlockToIndex, IndexerError> {
        match self.blocks.entry(block_height) {
            Entry::Vacant(entry) => {
                let block = BlockToIndex::load_from_disk(self.indexer.block_path(block_height))?;
                Ok(entry.insert(block))
            },
            Entry::Occupied(entry) => Ok(entry.into_mut()),
        }
    }
}

pub struct FileTransactionDataLoader {
    pub indexer: IndexerPath,
}

impl Loader<Transaction> for FileTransactionDataLoader {
    type Error = Arc<anyhow::Error>;
    type Value = (Tx, TxOutcome);

    async fn load(
        &self,
        keys: &[Transaction],
    ) -> Result<HashMap<Transaction, Self::Value>, Self::Error> {
        let mut cache = BlockCache::new(&self.indexer);

        keys.iter()
            .map(|graphql_tx| {
                // Load the block.
                let indexed_block = cache.get_or_load_from_disk(graphql_tx.block_height)?;

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
