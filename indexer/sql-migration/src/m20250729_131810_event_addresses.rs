use {
    crate::idens::EventAddress,
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
                    .table(EventAddress::Table)
                    .if_not_exists()
                    .col(pk_uuid(EventAddress::Id))
                    .col(
                        ColumnDef::new(EventAddress::BlockHeight)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(uuid(EventAddress::EventId))
                    .col(string(EventAddress::Address))
                    .to_owned(),
            )
            .await?;

        // Index for filtering by block height
        manager
            .create_index(
                Index::create()
                    .name("event_addresses-block_height")
                    .table(EventAddress::Table)
                    .col(EventAddress::BlockHeight)
                    .to_owned(),
            )
            .await?;

        // Index for filtering by address
        manager
            .create_index(
                Index::create()
                    .name("event_addresses-address")
                    .table(EventAddress::Table)
                    .col(EventAddress::Address)
                    .to_owned(),
            )
            .await?;

        // Index for filtering by block height and address
        manager
            .create_index(
                Index::create()
                    .name("event_addresses-block_height_address")
                    .table(EventAddress::Table)
                    .col(EventAddress::BlockHeight)
                    .col(EventAddress::Address)
                    .to_owned(),
            )
            .await?;

        // Index for joining with events table
        manager
            .create_index(
                Index::create()
                    .name("event_addresses-event_id")
                    .table(EventAddress::Table)
                    .col(EventAddress::EventId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use sea_orm_migration::{
        MigrationTrait, MigratorTrait, async_trait,
        prelude::{Alias, IntoIden},
        sea_orm::{self, Database},
    };

    pub struct Migrator;

    #[async_trait::async_trait]
    impl MigratorTrait for Migrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            vec![Box::new(super::Migration)]
        }

        fn migration_table_name() -> sea_orm::DynIden {
            Alias::new("grug_seaql_migrations").into_iden()
        }
    }

    #[tokio::test]
    #[ignore]
    async fn generate_entities() -> Result<(), Box<dyn std::error::Error>> {
        let uri = "postgres://postgres@localhost:5432/glaxe_bot_db";
        let db = Database::connect(uri).await?;

        println!("🔄 Starting migration");

        Migrator::up(&db, None).await?;

        println!("✅ Migration complete");

        let status = Command::new("sea-orm-cli")
            .args([
                "generate",
                "entity",
                "-u",
                uri,
                "-o",
                "./myentities",
                "--with-serde",
                "both",
            ])
            .status()?;

        if !status.success() {
            eprintln!("❌ Error generating entities");
            std::process::exit(1);
        }

        println!("✅ Database removed");

        println!("✅ Entities generated");

        Ok(())
    }
}
