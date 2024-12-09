use {
    crate::{active_model::Models, bail, entity, error},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{BlockInfo, BorshDeExt, BorshSerExt, Tx, TxOutcome},
    sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter},
    serde::{Deserialize, Serialize},
    std::{
        io::Write,
        path::{Path, PathBuf},
    },
    tempfile::NamedTempFile,
};

/// Saves the block and its transactions in memory
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct BlockToIndex {
    pub block_info: BlockInfo,
    pub txs: Vec<(Tx, TxOutcome)>,
    /// Where the block is temporarily saved on disk. I use `String` instead of `PathBuf` because
    /// `PathBuf` can not be serialized by Borsh and using `PathBuf` with #[borsh(skip)] means
    /// a default value will be set.
    filename: String,
}

impl BlockToIndex {
    pub fn new(block_info: BlockInfo, filename: String) -> Self {
        Self {
            block_info,
            txs: Vec::new(),
            filename,
        }
    }

    /// Takes care of inserting the data in the database in a single DB transaction
    pub async fn save(&self, db: &DatabaseTransaction) -> error::Result<()> {
        #[cfg(feature = "tracing")]
        tracing::info!(block_height = self.block_info.height, "Indexing block");

        let mut models = Models::build(&self.block_info)?;
        for (tx, tx_outcome) in self.txs.iter() {
            models.push(tx, tx_outcome)?;
        }

        // I check if the block already exists, if so it means we can skip the
        // whole block, transactions, messages and events since those are created
        // within a single DB transaction.
        // This scenario could happen if the process has crashed after block was
        // indexed but before the tmp_file was removed.
        let existing_block = entity::blocks::Entity::find()
            .filter(entity::blocks::Column::BlockHeight.eq(self.block_info.height))
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

    pub fn delete_tmp_file(&self) -> error::Result<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!(path = self.filename, "Removing block tmp_file");

        if let Err(_err) = std::fs::remove_file(&self.filename) {
            #[cfg(feature = "tracing")]
            tracing::warn!(path = self.filename, block_height = self.block_info.height, error = %_err, "Can't remove block tmp_file");
        }
        Ok(())
    }

    pub fn save_tmp_file(&self) -> error::Result<()> {
        let path = Path::new(&self.filename);
        let Some(directory) = path.parent() else {
            bail!("Can't detect parent directory");
        };
        let mut tmp_filename = NamedTempFile::new_in(directory)?;
        let encoded_block = self.to_borsh_vec()?;
        tmp_filename.write_all(&encoded_block)?;
        tmp_filename.flush()?;

        // TODO: look at the security implications of using `persist`
        tmp_filename.persist(&self.filename)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(path = %self.filename, block_height = self.block_info.height, "Saved tmp_file");
        Ok(())
    }

    pub fn load_tmp_file(filename: &PathBuf) -> error::Result<Self> {
        let data = std::fs::read(filename)?;
        Ok(data.deserialize_borsh()?)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, assertor::*, grug_types::Hash};

    #[test]
    fn test_save_and_load_and_delete_tmp_file() {
        let block_info = BlockInfo {
            height: 10,
            timestamp: Default::default(),
            hash: Hash::ZERO,
        };

        let temp_file = NamedTempFile::new().expect("Failed to create a temp file");
        let filename = temp_file.path().to_string_lossy().to_string();

        let block_to_index = BlockToIndex::new(block_info, filename);

        assert_that!(block_to_index.save_tmp_file()).is_ok();

        let saved_block_to_index =
            BlockToIndex::load_tmp_file(&temp_file.path().into()).expect("Can't load tmp file");

        assert_that!(saved_block_to_index).is_equal_to(block_to_index);
        assert_that!(saved_block_to_index.delete_tmp_file()).is_ok();
    }
}
