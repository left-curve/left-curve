use {
    dango_bridge_relayer_httpd::{error::Error, server},
    sea_orm::Database,
    std::env,
};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // get env vars
    dotenvy::dotenv().ok();
    let dango_url = env::var("DANGO_URL").expect("DANGO_URL must be set");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Get bridge config from Dango
    let bridge_config = server::get_bridge_config(dango_url).await?;

    // Create database connection
    let db = Database::connect(database_url).await?;

    server::run_servers(bridge_config, db).await?;

    Ok(())
}
