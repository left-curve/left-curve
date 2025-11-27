use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DepositAddress::Table)
                    .if_not_exists()
                    .col(pk_auto(DepositAddress::Id))
                    .col(string(DepositAddress::Address).unique_key())
                    .col(date_time(DepositAddress::CreatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DepositAddress::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DepositAddress {
    Table,
    Id,
    Address,
    CreatedAt,
}
