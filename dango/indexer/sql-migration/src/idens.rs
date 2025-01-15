use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub enum Swap {
    #[sea_orm(iden = "swaps")]
    Table,
    Id,
    BlockHeight,
    CreatedAt,
}
