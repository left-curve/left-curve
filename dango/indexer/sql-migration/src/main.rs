use {dango_indexer_sql_migration::Migrator, sea_orm_migration::prelude::*};

#[async_std::main]
async fn main() {
    cli::run_cli(Migrator).await;
}
