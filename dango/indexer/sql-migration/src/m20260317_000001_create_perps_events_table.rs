use {
    crate::idens::PerpsEvent,
    sea_orm_migration::{prelude::*, schema::*},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PerpsEvent::Table)
                    .if_not_exists()
                    .col(pk_uuid(PerpsEvent::Id))
                    .col(integer(PerpsEvent::Idx))
                    .col(
                        ColumnDef::new(PerpsEvent::BlockHeight)
                            .big_integer()
                            .not_null(),
                    )
                    .col(string(PerpsEvent::TxHash))
                    .col(date_time(PerpsEvent::CreatedAt))
                    .col(string(PerpsEvent::EventType))
                    .col(string(PerpsEvent::UserAddr))
                    .col(string(PerpsEvent::PairId))
                    .col(json_binary(PerpsEvent::Data))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("idx_perps_events_user_block")
                    .table(PerpsEvent::Table)
                    .col(PerpsEvent::UserAddr)
                    .col((PerpsEvent::BlockHeight, IndexOrder::Desc))
                    .col((PerpsEvent::Idx, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("idx_perps_events_block")
                    .table(PerpsEvent::Table)
                    .col(PerpsEvent::BlockHeight)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("idx_perps_events_type")
                    .table(PerpsEvent::Table)
                    .col(PerpsEvent::EventType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .if_not_exists()
                    .name("idx_perps_events_pair_id")
                    .table(PerpsEvent::Table)
                    .col(PerpsEvent::PairId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PerpsEvent::Table).to_owned())
            .await
    }
}
