use {super::idens::Transaction, sea_orm::DatabaseBackend, sea_orm_migration::prelude::*};

const INDEX_NAME: &str = "transactions_sender_block_height_transaction_idx";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_database_backend() {
            DatabaseBackend::Postgres => {
                manager
                    .get_connection()
                    .execute_unprepared(&format!(
                        "CREATE INDEX IF NOT EXISTS {INDEX_NAME} ON transactions (sender, block_height DESC, transaction_idx DESC)"
                    ))
                    .await?;
            },
            _ => {
                manager
                    .create_index(
                        sea_query::Index::create()
                            .if_not_exists()
                            .name(INDEX_NAME)
                            .table(Transaction::Table)
                            .col((Transaction::Sender, IndexOrder::Asc))
                            .col((Transaction::BlockHeight, IndexOrder::Desc))
                            .col((Transaction::TransactionIdx, IndexOrder::Desc))
                            .to_owned(),
                    )
                    .await?;
            },
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                sea_query::Index::drop()
                    .name(INDEX_NAME)
                    .table(Transaction::Table)
                    .to_owned(),
            )
            .await
    }
}
