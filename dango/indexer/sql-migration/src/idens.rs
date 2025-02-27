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
