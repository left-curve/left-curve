use {
    crate::activity::idens::ActivityEvents, sea_orm::ConnectionTrait, sea_orm_migration::prelude::*,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ActivityEvents::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ActivityEvents::Address).binary().not_null())
                    .col(
                        ColumnDef::new(ActivityEvents::BlockHeight)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivityEvents::Category)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivityEvents::CategoryIndex)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivityEvents::EventIndex)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ActivityEvents::EventType)
                            .small_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(ActivityEvents::Contract).binary().null())
                    .col(
                        ColumnDef::new(ActivityEvents::ContractEventName)
                            .text()
                            .null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ActivityEvents::Address)
                            .col(ActivityEvents::BlockHeight)
                            .col(ActivityEvents::Category)
                            .col(ActivityEvents::CategoryIndex)
                            .col(ActivityEvents::EventIndex),
                    )
                    .to_owned(),
            )
            .await?;

        // Six partial indexes, each NULL-filtered on the attribute (which also
        // skips the sentinel rows). The `(address, attr, …)` trio makes
        // "<attr> events involving X" a single seek; the `(attr, …, address)`
        // trio makes the address-less attribute feeds a backward scan +
        // `DISTINCT ON` over the position tail (address last → an event's
        // participant rows are adjacent, so the dedup is local). The recency /
        // keyset tail is the event position `(block_height, category,
        // category_index, event_index)`. Partial indexes are supported on both
        // Postgres and SQLite, so one raw statement serves every backend.
        let conn = manager.get_connection();
        let indexes: [(&str, &str, &str); 6] = [
            (
                "idx_activity_events_addr_contract",
                "address, contract, block_height, category, category_index, event_index",
                "contract",
            ),
            (
                "idx_activity_events_addr_name",
                "address, contract_event_name, block_height, category, category_index, event_index",
                "contract_event_name",
            ),
            (
                "idx_activity_events_addr_type",
                "address, event_type, block_height, category, category_index, event_index",
                "event_type",
            ),
            (
                "idx_activity_events_type",
                "event_type, block_height, category, category_index, event_index, address",
                "event_type",
            ),
            (
                "idx_activity_events_contract",
                "contract, block_height, category, category_index, event_index, address",
                "contract",
            ),
            (
                "idx_activity_events_name",
                "contract_event_name, block_height, category, category_index, event_index, address",
                "contract_event_name",
            ),
        ];
        for (name, cols, partial) in indexes {
            conn.execute_unprepared(&format!(
                "CREATE INDEX IF NOT EXISTS {name} ON activity_events ({cols}) WHERE {partial} IS NOT NULL"
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ActivityEvents::Table).to_owned())
            .await
    }
}
