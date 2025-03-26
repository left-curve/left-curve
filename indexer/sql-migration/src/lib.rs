// use sea_orm_migration::MigrationStatus;
pub use sea_orm_migration::prelude::*;
mod idens;

mod m20220101_000001_create_table;
mod m20250326_105930_blocks_proposer_address;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20250326_105930_blocks_proposer_address::Migration),
        ]
    }

    fn migration_table_name() -> sea_orm::DynIden {
        Alias::new("grug_seaql_migrations").into_iden()
    }
}

// #[async_trait::async_trait]
// impl MigratorExecuteTrait for Migrator {
//     fn migrations() -> Vec<Box<dyn MigrationExecuteTrait>> {
//         vec![
//             // Box::new(m20220101_000001_create_table::Migration),
//             // Box::new(m20250326_105930_blocks_proposer_address::Migration),
//         ]
//     }
// }

// /// The migration definition
// #[async_trait::async_trait]
// pub trait MigrationExecuteTrait: MigrationName + Send + Sync {
//     /// Define actions to perform when applying the migration
//     async fn execute(&self, db: &dyn ConnectionTrait) -> Result<(), DbErr>;
// }

// pub struct Migration {
//     migration: Box<dyn MigrationExecuteTrait>,
//     status: MigrationStatus,
// }

// /// Performing migrations on a database
// #[async_trait::async_trait]
// pub trait MigratorExecuteTrait: Send {
//     /// Vector of migrations in time sequence
//     fn migrations() -> Vec<Box<dyn MigrationExecuteTrait>>;

//     /// Get list of migrations wrapped in `Migration` struct
//     fn get_migration_files() -> Vec<Migration> {
//         Self::migrations()
//             .into_iter()
//             .map(|migration| Migration {
//                 migration,
//                 status: MigrationStatus::Pending,
//             })
//             .collect()
//     }
// }
