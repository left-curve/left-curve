use {
    crate::idens::{Account, AccountUser, PublicKey, User},
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
                    .table(User::Table)
                    .if_not_exists()
                    .col(pk_uuid(User::Id))
                    .col(string_uniq(User::Username))
                    .col(date_time(User::CreatedAt))
                    .col(
                        ColumnDef::new(User::CreatedBlockHeight)
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
                    .name("users-username")
                    .table(User::Table)
                    .col(User::Username)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Account::Table)
                    .if_not_exists()
                    .col(pk_uuid(Account::Id))
                    .col(integer(Account::AccountIndex))
                    .col(string(Account::Address))
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
                    .name("accounts-address")
                    .table(Account::Table)
                    .col(Account::Address)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AccountUser::Table)
                    .if_not_exists()
                    .col(pk_uuid(AccountUser::Id))
                    .col(uuid(AccountUser::AccountId))
                    .col(uuid(AccountUser::UserId))
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk_account_users_account_id")
                            .from_tbl(AccountUser::Table)
                            .from_col(AccountUser::AccountId)
                            .to_tbl(Account::Table)
                            .to_col(Account::Id),
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk_account_users_user_id")
                            .from_tbl(AccountUser::Table)
                            .from_col(AccountUser::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PublicKey::Table)
                    .if_not_exists()
                    .col(pk_uuid(PublicKey::Id))
                    .col(string(PublicKey::Username))
                    .col(string(PublicKey::KeyHash))
                    .col(string(PublicKey::PublicKey))
                    .col(small_integer(PublicKey::KeyType))
                    .col(date_time(PublicKey::CreatedAt))
                    .col(
                        ColumnDef::new(PublicKey::CreatedBlockHeight)
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
                    .name("public_keys-keyhash")
                    .table(PublicKey::Table)
                    .col(PublicKey::KeyHash)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PublicKey::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Account::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        Ok(())
    }
}
