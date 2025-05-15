use {
    crate::graphql::types::transaction::Transaction,
    anyhow::anyhow,
    async_graphql::dataloader::Loader,
    grug_types::{Tx, TxOutcome},
    indexer_sql::{block_to_index::BlockToIndex, indexer_path::IndexerPath},
    std::{collections::HashMap, sync::Arc},
};

pub struct FileTranscationDataLoader {
    pub indexer: IndexerPath,
}

impl Loader<Transaction> for FileTranscationDataLoader {
    type Error = Arc<anyhow::Error>;
    type Value = (Tx, TxOutcome);

    async fn load(
        &self,
        keys: &[Transaction],
    ) -> Result<HashMap<Transaction, Self::Value>, Self::Error> {
        let mut buff = HashMap::<u64, BlockToIndex>::new();

        keys.into_iter()
            .filter_map(|graphql_tx| {
                let indexed_block = match buff.get(&graphql_tx.block_height) {
                    Some(block_to_index) => block_to_index.clone(),
                    None => {
                        let block_to_index = match BlockToIndex::load_from_disk(
                            self.indexer.block_path(graphql_tx.block_height),
                        ) {
                            Ok(b) => b,
                            Err(err) => {
                                return Some(Err(err.into()));
                            },
                        };

                        buff.insert(graphql_tx.block_height, block_to_index.clone());
                        block_to_index
                    },
                };

                let tx = if let Some((tx, hash)) = indexed_block
                    .block
                    .txs
                    .get(graphql_tx.transaction_idx as usize)
                {
                    if hash.to_string() == graphql_tx.hash {
                        tx.clone()
                    } else {
                        return Some(Err(anyhow!(
                            "Transaction hash mismatch: {} != {}",
                            hash.to_string(),
                            graphql_tx.hash
                        )));
                    }
                } else {
                    return Some(Err(anyhow!("Transaction not found: {}", graphql_tx.hash)));
                };

                let outcome = if let Some(outcome) = indexed_block
                    .block_outcome
                    .tx_outcomes
                    .get(graphql_tx.transaction_idx as usize)
                {
                    outcome.clone()
                } else {
                    return Some(Err(anyhow!(
                        "Transaction outcome not found: {}",
                        graphql_tx.hash
                    )));
                };

                Some(Ok((graphql_tx.clone(), (tx, outcome))))
            })
            .collect::<Result<_, _>>()
            .map_err(Arc::new)
    }
}
