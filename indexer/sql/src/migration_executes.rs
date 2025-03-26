use std::collections::HashMap;

pub use sea_orm_migration::prelude::*;
use {grug_app::IndexerBatch, indexer_sql_migration::MigrationName, sea_orm::DatabaseConnection};

mod m20250326_105930_blocks_proposer_address;

pub struct ExecuteMigrator;

#[async_trait::async_trait]
impl MigratorExecuteTrait for ExecuteMigrator {
    fn migrations() -> HashMap<String, Box<dyn MigrationExecuteTrait>> {
        // Add the execution of the migration files here, name should be
        // the same as the `sql-migration` files, and code will be executed
        // after the migration has ran.
        let files: Vec<Box<dyn MigrationExecuteTrait>> = vec![Box::new(
            m20250326_105930_blocks_proposer_address::Migration,
        )];

        files
            .into_iter()
            .map(|file| (file.name().to_string(), file))
            .collect()
    }
}

#[async_trait::async_trait]
pub trait MigrationExecuteTrait: MigrationName + Send + Sync {
    /// Define actions to perform after applying the migration
    async fn post_execute(
        &self,
        db: &DatabaseConnection,
        indexer: &(dyn IndexerBatch + Sync),
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Performing code executions after migrations
#[async_trait::async_trait]
pub trait MigratorExecuteTrait: Send {
    /// Code executions connected to the migration names
    fn migrations() -> HashMap<String, Box<dyn MigrationExecuteTrait>>;
}
