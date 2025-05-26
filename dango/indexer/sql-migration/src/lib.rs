pub use sea_orm_migration::prelude::*;
mod idens;

mod m20250115_000001_create_table;
mod m20250328_111712_accounts_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250115_000001_create_table::Migration),
            Box::new(m20250328_111712_accounts_table::Migration),
        ]
    }

    fn migration_table_name() -> sea_orm::DynIden {
        Alias::new("dango_seaql_migrations").into_iden()
    }
}
