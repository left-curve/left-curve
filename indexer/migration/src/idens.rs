use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub enum Block {
    #[sea_orm(iden = "blocks")]
    Table,
    Id,
    CreatedAt,
    BlockHeight,
}

#[derive(DeriveIden)]
pub enum Transaction {
    #[sea_orm(iden = "transactions")]
    Table,
    Id,
    Hash,
    HasSucceeded,
    ErrorMessage,
    BlockHeight,
    GasWanted,
    GasUsed,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum Message {
    #[sea_orm(iden = "messages")]
    Table,
    Id,
    Data,
    BlockHeight,
    CreatedAt,
}
