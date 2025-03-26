// use sea_orm_migration::MigrationStatus;
pub use sea_orm_migration::prelude::*;
mod idens;

mod m20220101_000001_create_table;
mod m20250326_105930_blocks_proposer_address;
mod m20250326_145333_blocks_proposer_address;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20250326_105930_blocks_proposer_address::Migration),
            Box::new(m20250326_145333_blocks_proposer_address::Migration),
        ]
    }

    fn migration_table_name() -> sea_orm::DynIden {
        Alias::new("grug_seaql_migrations").into_iden()
    }
}
