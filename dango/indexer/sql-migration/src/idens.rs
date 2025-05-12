use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub enum Transfer {
    #[sea_orm(iden = "transfers")]
    Table,
    Id,
    Idx,
    BlockHeight,
    CreatedAt,
    FromAddress,
    ToAddress,
    Amount,
    Denom,
}

#[derive(DeriveIden)]
pub enum Account {
    #[sea_orm(iden = "accounts")]
    Table,
    Id,
    Username,
    // Index,
    #[allow(clippy::enum_variant_names)]
    AccountType,
    // Can this be changed later?
    Address,
    // NOTE: should it be PublicKey or just removed?
    EthAddress,
    CreatedBlockHeight,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum AccountsPublicKeys {
    #[sea_orm(iden = "accounts_public_keys")]
    Table,
    AccountId,
    PublicKeyId,
}

#[derive(DeriveIden)]
pub enum PublicKeys {
    #[sea_orm(iden = "public_keys")]
    Table,
    Id,
    // Unique index
    PublicKey,
}
