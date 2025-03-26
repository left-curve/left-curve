use std::collections::HashMap;

pub use sea_orm_migration::prelude::*;
use {grug_app::IndexerBatch, indexer_sql_migration::MigrationName, sea_orm::DatabaseConnection};

mod m20250326_105930_blocks_proposer_address;

pub struct ExecuteMigrator;

#[async_trait::async_trait]
impl MigratorExecuteTrait for ExecuteMigrator {
    fn migrations() -> HashMap<String, Box<dyn MigrationExecuteTrait>> {
        let files: Vec<Box<dyn MigrationExecuteTrait>> = vec![Box::new(
            m20250326_105930_blocks_proposer_address::Migration,
        )];

        files
            .into_iter()
            .map(|file| (file.name().to_string(), file))
            .collect()
    }
}

/// The migration definition
#[async_trait::async_trait]
pub trait MigrationExecuteTrait: MigrationName + Send + Sync {
    /// Define actions to perform when applying the migration
    async fn execute(
        &self,
        db: &DatabaseConnection,
        indexer: &(dyn IndexerBatch + Sync),
    ) -> Result<(), Box<dyn std::error::Error>>;
}

// pub struct Migration {
//     migration: Box<dyn MigrationExecuteTrait>,
//     status: MigrationStatus,
// }

/// Performing migrations on a database
#[async_trait::async_trait]
pub trait MigratorExecuteTrait: Send {
    /// Vector of migrations in time sequence
    fn migrations() -> HashMap<String, Box<dyn MigrationExecuteTrait>>;

    // /// Get list of migrations wrapped in `Migration` struct
    // fn get_migration_files() -> Vec<Migration> {
    //     Self::migrations()
    //         .into_iter()
    //         .map(|(_, migration)| Migration {
    //             migration,
    //             status: MigrationStatus::Pending,
    //         })
    //         .collect()
    // }
}
