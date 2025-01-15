use {
    crate::idens::Swap,
    sea_orm_migration::{prelude::*, schema::*},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Swap::Table)
                    .if_not_exists()
                    .col(pk_uuid(Swap::Id))
                    .col(date_time(Swap::CreatedAt))
                    .col(
                        ColumnDef::new(Swap::BlockHeight)
                            .big_unsigned()
                            .unique_key()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Swap::Table).to_owned())
            .await?;
        Ok(())
    }
}
