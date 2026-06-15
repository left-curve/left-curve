use {super::idens::Message, sea_orm_migration::prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("messages_transaction_id_idx")
                    .table(Message::Table)
                    .col(Message::TransactionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                sea_query::Index::drop()
                    .name("messages_transaction_id_idx")
                    .table(Message::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
