pub use sea_orm_migration::prelude::*;
mod idens;

mod m20220101_000001_create_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20220101_000001_create_table::Migration)]
    }

    fn migration_table_name() -> sea_orm::DynIden {
        Alias::new("grug_seaql_migrations").into_iden()
    }
}
