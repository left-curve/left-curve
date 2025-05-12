use {
    crate::idens::Account,
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
                    .table(Account::Table)
                    .if_not_exists()
                    .col(pk_uuid(Account::Id))
                    .col(string_uniq(Account::Username))
                    // .col(integer(Account::Index))
                    .col(string_uniq(Account::Address))
                    .col(string_null(Account::EthAddress))
                    .col(small_integer(Account::AccountType))
                    .col(date_time(Account::CreatedAt))
                    .col(
                        ColumnDef::new(Account::CreatedBlockHeight)
                            .big_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("accounts-username")
                    .table(Account::Table)
                    .col(Account::Username)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("accounts-address")
                    .table(Account::Table)
                    .col(Account::Address)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Account::Table).to_owned())
            .await?;

        Ok(())
    }
}
