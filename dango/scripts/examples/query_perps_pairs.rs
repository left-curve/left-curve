use {
    dango_types::{
        config::AppConfig,
        perps::{self, PairId, PairParam},
    },
    grug::QueryClientExt,
    indexer_client::HttpClient,
    std::collections::BTreeMap,
};

/// Queries all perps pair IDs from the testnet API and prints them.
///
/// Usage:
///   cargo run --example query_perps_pairs
#[tokio::main]
async fn main() {
    let client = HttpClient::new("https://api-testnet.dango.zone").unwrap();

    // Fetch the app config to obtain the perps contract address.
    let app_config: AppConfig = client.query_app_config(None).await.unwrap();
    let perps_addr = app_config.addresses.perps;

    println!("perps contract: {perps_addr}");

    // Query all pair parameters (paginated; first page with default limit).
    let pair_params: BTreeMap<PairId, PairParam> = client
        .query_wasm_smart(
            perps_addr,
            perps::QueryPairParamsRequest {
                start_after: None,
                limit: None,
            },
            None,
        )
        .await
        .unwrap();

    println!("found {} pair(s):", pair_params.len());
    for pair_id in pair_params.keys() {
        println!("  {pair_id}");
    }
}
