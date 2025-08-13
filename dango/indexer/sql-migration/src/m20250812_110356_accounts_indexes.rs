use {
    super::idens::{Account, AccountUser},
    sea_orm_migration::prelude::*,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("accounts_users_account_user_idx")
                    .table(AccountUser::Table)
                    .col(AccountUser::AccountId)
                    .col(AccountUser::UserId)
                    .to_owned(),
            )
            .await?;
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

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                sea_query::Index::drop()
                    .name("accounts_users_account_user_idx")
                    .table(AccountUser::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                sea_query::Index::drop()
                    .name("accounts_block_type_idx")
                    .table(Account::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
