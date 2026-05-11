use {crate::idens::PerpsEvent, sea_orm::DatabaseBackend, sea_orm_migration::prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_database_backend() {
            DatabaseBackend::Postgres => {
                manager
                    .get_connection()
                    .execute_unprepared(
                        "CREATE INDEX IF NOT EXISTS idx_perps_events_created_at ON perps_events USING brin (created_at)",
                    )
                    .await?;
            },
            _ => {
                manager
                    .create_index(
                        sea_query::Index::create()
                            .if_not_exists()
                            .name("idx_perps_events_created_at")
                            .table(PerpsEvent::Table)
                            .col(PerpsEvent::CreatedAt)
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
                    .name("idx_perps_events_created_at")
                    .table(PerpsEvent::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
