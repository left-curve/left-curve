pub use sea_orm_migration::prelude::*;
mod idens;

mod m20220101_000001_create_table;
mod m20250324_094658_blocks_transactions_count;
mod m20250529_142838_transactions_sender_index;
mod m20250717_192805_events_data_index;
mod m20250812_110949_messages_indexes;
mod m20250902_175929_transactions_http_request_details;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20250324_094658_blocks_transactions_count::Migration),
            Box::new(m20250529_142838_transactions_sender_index::Migration),
            Box::new(m20250717_192805_events_data_index::Migration),
            Box::new(m20250812_110949_messages_indexes::Migration),
            Box::new(m20250902_175929_transactions_http_request_details::Migration),
        ]
    }

    fn migration_table_name() -> sea_orm::DynIden {
        Alias::new("grug_seaql_migrations").into_iden()
    }
}
