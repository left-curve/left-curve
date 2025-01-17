use {
    crate::idens::Transfer,
    sea_orm::DatabaseBackend,
    sea_orm_migration::{prelude::*, schema::*},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // For later, we can use this to add support for different databases and
        // keep numeric for psql but text for sqlite
        match manager.get_database_backend() {
            DatabaseBackend::Sqlite => {
                println!("Running on SQLite");
            },
            _ => {
                println!("Not running on SQLite");
            },
        }

        manager
            .create_table(
                Table::create()
                    .table(Transfer::Table)
                    .if_not_exists()
                    .col(pk_uuid(Transfer::Id))
                    .col(date_time(Transfer::CreatedAt))
                    .col(
                        ColumnDef::new(Transfer::BlockHeight)
                            .big_unsigned()
                            .unique_key()
                            .not_null(),
                    )
                    .col(string(Transfer::FromAddress))
                    .col(string(Transfer::ToAddress))
                    // SQLite doesn't support decimal_len for such a large number :(
                    // .col(decimal_len(Transfer::Amount, 39, 0))
                    .col(string(Transfer::Amount))
                    .col(string(Transfer::Denom))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("transfers-block_height")
                    .table(Transfer::Table)
                    .col(Transfer::BlockHeight)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Transfer::Table).to_owned())
            .await?;
        Ok(())
    }
}
