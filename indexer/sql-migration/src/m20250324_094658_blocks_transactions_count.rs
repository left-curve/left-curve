use {
    crate::idens::Block,
    sea_orm_migration::{prelude::*, sea_orm::query::*},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Block::Table)
                    .add_column(
                        ColumnDef::new(Block::TransactionsCount)
                            .unsigned()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        // Fill up the transactions_count field for each block in the database.
        let db = manager.get_connection();
        let transaction = db.begin().await?;
        let query = r#"
-- Step 1: create a temporary table
CREATE TEMP TABLE tmp_counts AS
SELECT block_height, COUNT(*) AS transactions_count
FROM transactions
GROUP BY block_height;

-- Step 2: update using the temp table
UPDATE blocks
SET transactions_count = (
  SELECT transactions_count
  FROM tmp_counts
  WHERE tmp_counts.block_height = blocks.block_height
);

-- Step 3: drop the temporary table
DROP TABLE tmp_counts;
"#;
        transaction.execute_unprepared(query).await?;
        transaction.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Block::Table)
                    .drop_column(Block::TransactionsCount)
                    .to_owned(),
            )
            .await
    }
}
