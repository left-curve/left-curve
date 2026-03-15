use {super::idens::Account, sea_orm_migration::prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the composite index that references account_type.
        manager
            .drop_index(
                sea_query::Index::drop()
                    .name("accounts_block_type_idx")
                    .table(Account::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the account_type column.
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .drop_column(Account::AccountType)
                    .to_owned(),
            )
            .await?;

        // Create a replacement index on just block height.
        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("accounts_block_height_idx")
                    .table(Account::Table)
                    .col(Account::CreatedBlockHeight)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the block-height-only index.
        manager
            .drop_index(
                sea_query::Index::drop()
                    .name("accounts_block_height_idx")
                    .table(Account::Table)
                    .to_owned(),
            )
            .await?;

        // Re-add account_type column with a default of 0 (Single).
        manager
            .alter_table(
                Table::alter()
                    .table(Account::Table)
                    .add_column(
                        ColumnDef::new(Account::AccountType)
                            .small_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        // Recreate the composite index.
        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("accounts_block_type_idx")
                    .table(Account::Table)
                    .col(Account::CreatedBlockHeight)
                    .col(Account::AccountType)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
