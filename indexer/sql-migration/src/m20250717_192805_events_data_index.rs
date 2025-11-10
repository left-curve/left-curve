use {crate::idens::Event, sea_orm_migration::prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("events-data")
                    .table(Event::Table)
                    .col(Event::Data)
                    .full_text()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("events-data")
                    .table(Event::Table)
                    .to_owned(),
            )
            .await
    }
}
