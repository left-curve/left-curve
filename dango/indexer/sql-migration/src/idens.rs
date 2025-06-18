use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub enum Transfer {
    #[sea_orm(iden = "transfers")]
    Table,
    Id,
    Idx,
    BlockHeight,
    TxHash,
    CreatedAt,
    FromAddress,
    ToAddress,
    Amount,
    Denom,
}

#[derive(DeriveIden)]
pub enum User {
    #[sea_orm(iden = "users")]
    Table,
    Id,
    Username,
    CreatedBlockHeight,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum Account {
    #[sea_orm(iden = "accounts")]
    Table,
    Id,
    #[allow(clippy::enum_variant_names)]
    AccountIndex,
    #[allow(clippy::enum_variant_names)]
    AccountType,
    Address,
    CreatedBlockHeight,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum AccountUser {
    #[sea_orm(iden = "accounts_users")]
    Table,
    Id,
    AccountId,
    UserId,
}

#[derive(DeriveIden)]
pub enum PublicKey {
    #[sea_orm(iden = "users_public_keys")]
    Table,
    Id,
    Username,
    #[allow(clippy::enum_variant_names)]
    PublicKey,
    KeyHash,
    KeyType,
    CreatedBlockHeight,
    CreatedAt,
}
