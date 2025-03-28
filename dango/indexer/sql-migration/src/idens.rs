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
    Index,
    #[allow(clippy::enum_variant_names)]
    AccountType,
    Address,
    EthAddress,
    CreatedBlockHeight,
    CreatedAt,
}
