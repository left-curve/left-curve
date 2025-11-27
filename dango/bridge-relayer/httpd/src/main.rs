use {
    dango_bridge_relayer_httpd::{context::Context, error::Error, migrations, server},
    dango_types::{
        bitcoin::{Config, MultisigSettings, Network, QueryConfigRequest},
        config::AppConfig,
    },
    grug::{
        __private::hex_literal::hex, Addr, ClientWrapper, HexByteArray, NonEmpty, QueryClientExt,
        Uint128, btree_set,
    },
    indexer_client::HttpClient,
    metrics_exporter_prometheus::PrometheusBuilder,
    sea_orm::Database,
    sea_orm_migration::MigratorTrait,
    std::{env, sync::Arc},
};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // get env vars
    dotenvy::dotenv().ok();
    let dango_url = env::var("DANGO_URL").expect("DANGO_URL must be set");
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Initialize metrics handler.
    // This should be done as soon as possible to capture all events.
    let metrics_handler = PrometheusBuilder::new().install_recorder()?;

    // Initialize Dango client.
    let dango_client = ClientWrapper::new(Arc::new(HttpClient::new(dango_url)?));

    // Query the bitcoin bridge contract address.
    // let bitcoin_bridge = dango_client
    //     .query_app_config::<AppConfig>(None)
    //     .await?
    //     .addresses
    //     .bitcoin;
    let bitcoin_bridge = Addr::mock(0);

    // Load bitcoin bridge config from contract.
    // let bridge_config = dango_client
    //     .query_wasm_smart(bitcoin_bridge, QueryConfigRequest {}, None)
    //     .await?;
    let pk1 = hex!("029ba1aeddafb6ff65d403d50c0db0adbb8b5b3616c3bc75fb6fecd075327099f6");
    let bridge_config = Config {
        network: Network::Testnet,
        vault: "0x0000000000000000000000000000000000000000".to_string(),
        multisig: MultisigSettings::new(
            1,
            NonEmpty::new(btree_set!(HexByteArray::from_inner(pk1))).unwrap(),
        )
        .unwrap(),
        sats_per_vbyte: Uint128::new(1),
        fee_rate_updater: Addr::mock(0),
        minimum_deposit: Uint128::new(1),
        max_output_per_tx: 1,
    };

    // Create database connection
    let db = Database::connect(database_url).await?;

    // Run migrations.
    #[cfg(feature = "tracing")]
    tracing::info!("running migrations");
    migrations::Migrator::up(&db, None).await?;
    #[cfg(feature = "tracing")]
    tracing::info!("ran migrations successfully");

    // Create context.
    let context = Context::new(bridge_config, db);

    // Run the server
    let server = server::run_server("127.0.0.1", 8080, None, context);
    let metrics = server::run_metrics_server("127.0.0.1", 8081, metrics_handler);

    tokio::try_join!(server, metrics)?;

    Ok(())
}
