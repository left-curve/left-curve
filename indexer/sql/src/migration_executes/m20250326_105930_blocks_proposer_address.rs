use {
    super::MigrationExecuteTrait,
    crate::entity,
    futures_util::TryStreamExt,
    grug_app::IndexerBatch,
    sea_orm::{
        ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, TransactionTrait,
    },
    sea_orm_migration::prelude::*,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationExecuteTrait for Migration {
    async fn execute(
        &self,
        db: &DatabaseConnection,
        indexer: &(dyn IndexerBatch + Sync),
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Using stream to avoid loading all blocks into memory
        let mut blocks = entity::blocks::Entity::find()
            .stream(db)
            .await
            .map_err(|err| err.to_string())?;

        // Stream uses a direct connection, and the db updates will use a transaction
        let txn = db.begin().await?;

        while let Ok(Some(db_block)) = blocks.try_next().await {
            if !db_block.proposer_address.is_empty() {
                continue;
            }

            let block = indexer
                .block(db_block.block_height as u64)
                .map_err(|err| err.to_string())?;

            // TODO: find the proposer
            let proposer_address = block.block.info.hash.to_string();

            let mut save_db_block: entity::blocks::ActiveModel = db_block.into();
            save_db_block.proposer_address = Set(proposer_address);
            save_db_block.update(&txn).await?;
        }

        drop(blocks);

        txn.commit().await?;

        Ok(())
    }
}
