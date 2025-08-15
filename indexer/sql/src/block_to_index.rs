#[cfg(feature = "metrics")]
use {metrics::counter, std::time::Instant};

use {
    crate::{active_model::Models, entity, error, event_cache::EventCacheWriter},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{Block, BlockOutcome},
    indexer_disk_saver::persistence::DiskPersistence,
    itertools::Itertools,
    sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait},
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

/// Saves the block and its transactions in memory
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct BlockToIndex {
    pub block: Block,
    pub block_outcome: BlockOutcome,
    #[serde(skip)]
    #[borsh(skip)]
    filename: PathBuf,
}

/// Maximum number of items to insert in a single `insert_many` operation.
/// This to avoid the following psql error:
/// PgConnection::run(): too many arguments for query
/// See discussion here:
/// https://www.postgresql.org/message-id/13394.1533697144%40sss.pgh.pa.us
pub const MAX_ROWS_INSERT: usize = 2048;

impl BlockToIndex {
    pub fn new(filename: PathBuf, block: Block, block_outcome: BlockOutcome) -> Self {
        Self {
            block,
            block_outcome,
            filename,
        }
    }

    /// Takes care of inserting the data in the database in a single DB transaction
    pub async fn save(
        &self,
        db: DatabaseConnection,
        event_cache: EventCacheWriter,
        #[allow(unused_variables)] indexer_id: u64,
    ) -> error::Result<()> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        #[cfg(feature = "tracing")]
        tracing::info!(
            block_height = self.block.info.height,
            indexer_id,
            "Indexing block"
        );

        let models = Models::build(&self.block, &self.block_outcome)?;

        let db = db.begin().await?;

        // I check if the block already exists, if so it means we can skip the
        // whole block, transactions, messages and events since those are created
        // within a single DB transaction.
        // This scenario could happen if the process has crashed after block was
        // indexed but before the tmp_file was removed.
        let existing_block = entity::blocks::Entity::find()
            .filter(entity::blocks::Column::BlockHeight.eq(self.block.info.height))
            .one(&db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!(err = %_e, "Failed to check if block exists");
            })?;

        if existing_block.is_some() {
            return Ok(());
        }

        entity::blocks::Entity::insert(models.block)
            .exec_without_returning(&db)
            .await
            .inspect_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::error!(err = %_e, "Failed to insert block");

                #[cfg(feature = "metrics")]
                counter!("indexer.database.errors.total").increment(1);
            })?;

        #[cfg(feature = "metrics")]
        {
            counter!("indexer.blocks.total").increment(1);
            counter!("indexer.transactions.total").increment(models.transactions.len() as u64);
            counter!("indexer.messages.total").increment(models.messages.len() as u64);
            counter!("indexer.events.total").increment(models.events.len() as u64);
        }

        if !models.transactions.is_empty() {
            #[cfg(feature = "tracing")]
            let transactions_len = models.transactions.len();

            for transactions in models
                .transactions
                .into_iter()
                .chunks(MAX_ROWS_INSERT)
                .into_iter()
                .map(|c| c.collect())
                .collect::<Vec<Vec<_>>>()
            {
                entity::transactions::Entity::insert_many(transactions)
                .exec_without_returning(&db)
                .await.inspect_err(|_e| {
                    #[cfg(feature = "tracing")]
                    tracing::error!(err = %_e, transactions_len=transactions_len, "Failed to insert transactions");

                    #[cfg(feature = "metrics")]
                    counter!("indexer.database.errors.total").increment(1);
                })?;
            }
        }

        if !models.messages.is_empty() {
            #[cfg(feature = "tracing")]
            let messages_len = models.messages.len();

            for messages in models
                .messages
                .into_iter()
                .chunks(MAX_ROWS_INSERT)
                .into_iter()
                .map(|c| c.collect())
                .collect::<Vec<Vec<_>>>()
            {
                entity::messages::Entity::insert_many(messages)
                .exec_without_returning(&db)
                .await.inspect_err(|_e| {
                    #[cfg(feature = "tracing")]
                    tracing::error!(err = %_e, messages_len=messages_len, "Failed to insert messages");

                    #[cfg(feature = "metrics")]
                    counter!("indexer.database.errors.total").increment(1);
                })?;
            }
        }

        if !models.events.is_empty() {
            #[cfg(feature = "tracing")]
            let events_len = models.events.len();

            for events in models
                .events
                .into_iter()
                .chunks(MAX_ROWS_INSERT)
                .into_iter()
                .map(|c| c.collect())
                .collect::<Vec<Vec<_>>>()
            {
                entity::events::Entity::insert_many(events)
                .exec_without_returning(&db)
                .await
                .inspect_err(|_e| {
                    #[cfg(feature = "tracing")]
                    tracing::error!(err = %_e, events_len=events_len, "Failed to insert events");

                    #[cfg(feature = "metrics")]
                    counter!("indexer.database.errors.total").increment(1);
                })?;
            }
        }

        db.commit().await?;

        event_cache
            .save_events(self.block.info.height, models.events_by_address)
            .await;

        #[cfg(feature = "metrics")]
        metrics::histogram!("indexer.block_save.duration").record(start.elapsed().as_secs_f64());

        Ok(())
    }
}

impl BlockToIndex {
    pub fn save_to_disk(&self) -> error::Result<()> {
        Ok(DiskPersistence::new(self.filename.clone(), false).save(self)?)
    }

    pub fn compress_file(file_path: PathBuf) -> error::Result<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!(?file_path, "Compressing block file");

        let mut file = DiskPersistence::new(file_path.clone(), false);

        #[allow(unused_variables)]
        let compressed = file.compress()?;

        #[cfg(feature = "tracing")]
        tracing::info!(
            ?file.file_path,
            ?compressed,
            "Compressed block file"
        );

        Ok(())
    }

    pub fn load_from_disk(file_path: PathBuf) -> error::Result<Self> {
        let mut block_to_index: Self = DiskPersistence::new(file_path.clone(), false).load()?;
        block_to_index.filename = file_path;
        Ok(block_to_index)
    }

    pub async fn load_from_disk_async(file_path: PathBuf) -> error::Result<Self> {
        tokio::task::spawn_blocking(move || Self::load_from_disk(file_path)).await?
    }

    pub fn exists(file_path: PathBuf) -> bool {
        DiskPersistence::new(file_path, false).exists()
    }

    pub fn delete_from_disk(file_path: PathBuf) -> error::Result<()> {
        Ok(DiskPersistence::new(file_path, false).delete()?)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        assertor::*,
        grug_types::{BlockInfo, Hash},
        tempfile::NamedTempFile,
    };

    #[test]
    fn test_save_and_load_and_delete_tmp_file() {
        let block_info = BlockInfo {
            height: 10,
            timestamp: Default::default(),
            hash: Hash::ZERO,
        };

        let block = Block {
            info: block_info,
            txs: vec![],
        };

        let block_outcome = BlockOutcome {
            height: 10,
            app_hash: Hash::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        let temp_file = NamedTempFile::new().expect("Failed to create a temp file");
        let temp_filename = temp_file.path().to_path_buf();

        let block_to_index = BlockToIndex::new(temp_filename.clone(), block, block_outcome);

        assert_that!(block_to_index.save_to_disk()).is_ok();

        let saved_block_to_index =
            BlockToIndex::load_from_disk(temp_filename.clone()).expect("Can't load tmp file");

        assert_that!(saved_block_to_index).is_equal_to(block_to_index);
        assert_that!(BlockToIndex::delete_from_disk(temp_filename)).is_ok();
    }
}
