use {
    crate::{active_model::Models, entity, error},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{Block, BlockOutcome},
    indexer_disk_saver::persistence::DiskPersistence,
    sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter},
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

impl BlockToIndex {
    pub fn new(filename: PathBuf, block: Block, block_outcome: BlockOutcome) -> Self {
        Self {
            block,
            block_outcome,
            filename,
        }
    }

    /// Takes care of inserting the data in the database in a single DB transaction
    pub async fn save<C: ConnectionTrait>(&self, db: &C) -> error::Result<()> {
        #[cfg(feature = "tracing")]
        tracing::info!(block_height = self.block.info.height, "Indexing block");

        let mut models = Models::build(&self.block, &self.block_outcome)?;

        for (tx, tx_outcome) in self
            .block
            .txs
            .iter()
            .zip(self.block_outcome.tx_outcomes.iter())
        {
            models.push(tx, tx_outcome)?;
        }

        // I check if the block already exists, if so it means we can skip the
        // whole block, transactions, messages and events since those are created
        // within a single DB transaction.
        // This scenario could happen if the process has crashed after block was
        // indexed but before the tmp_file was removed.
        let existing_block = entity::blocks::Entity::find()
            .filter(entity::blocks::Column::BlockHeight.eq(self.block.info.height))
            .one(db)
            .await?;

        if existing_block.is_some() {
            return Ok(());
        }

        entity::blocks::Entity::insert(models.block)
            .exec_without_returning(db)
            .await?;

        if !models.transactions.is_empty() {
            entity::transactions::Entity::insert_many(models.transactions)
                .exec_without_returning(db)
                .await?;
        }
        if !models.messages.is_empty() {
            entity::messages::Entity::insert_many(models.messages)
                .exec_without_returning(db)
                .await?;
        }
        if !models.events.is_empty() {
            entity::events::Entity::insert_many(models.events)
                .exec_without_returning(db)
                .await?;
        }

        Ok(())
    }
}

impl BlockToIndex {
    pub fn save_to_disk(&self) -> error::Result<()> {
        Ok(DiskPersistence::new(self.filename.clone(), false).save(self)?)
    }

    pub fn compress_file(file_path: PathBuf) -> error::Result<()> {
        DiskPersistence::new(file_path, false).compress()?;
        Ok(())
    }

    pub fn load_from_disk(file_path: PathBuf) -> error::Result<Self> {
        let mut block_to_index: Self = DiskPersistence::new(file_path.clone(), false).load()?;
        block_to_index.filename = file_path;
        Ok(block_to_index)
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
