use {crate::idens::Transaction, sea_orm_migration::prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Transaction::Table)
                    .add_column(ColumnDef::new(Transaction::HttpRequestDetails).json_binary())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Transaction::Table)
                    .drop_column(Transaction::HttpRequestDetails)
                    .to_owned(),
            )
            .await
    }
}
