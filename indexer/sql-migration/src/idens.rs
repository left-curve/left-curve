use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub enum Block {
    #[sea_orm(iden = "blocks")]
    Table,
    Id,
    #[allow(clippy::enum_variant_names)]
    BlockHeight,
    CreatedAt,
    Hash,
    AppHash,
}

#[derive(DeriveIden)]
pub enum Transaction {
    #[sea_orm(iden = "transactions")]
    Table,
    Id,
    Sender,
    Data,
    Credential,
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
    TransactionId,
    Data,
    MethodName,
    ContractAddr,
    SenderAddr,
    BlockHeight,
    CreatedAt,
}

#[derive(DeriveIden)]
pub enum Event {
    #[sea_orm(iden = "events")]
    Table,
    Id,
    TransactionId,
    Type,
    EventStatus,
    CommitmentStatus,
    Method,
    OrderId,
    // ContractAddr,
    Attributes,
    BlockHeight,
    CreatedAt,
}
