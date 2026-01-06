use {
    dango_types::{
        account_factory::{QueryUserRequest, UserIndexOrName},
        config::AppConfig,
    },
    grug::QueryClientExt,
    indexer_client::HttpClient,
};

// This script prints the address of the chain and the first 10 user accounts (by index) from the account factory
// (mostly used to update documentation).
#[tokio::main]
async fn main() {
    let client = HttpClient::new("https://api-devnet.dango.zone").unwrap();

    let app_config: AppConfig = client.query_app_config(None).await.unwrap();

    println!("{:#?}", app_config.addresses);

    let config = client.query_config(None).await.unwrap();
    println!("owner {}", config.owner);
    println!("bank {}", config.bank);

    // Query the user addresses
    for i in 1..10 {
        let (user_address, _) = client
            .query_wasm_smart(
                app_config.addresses.account_factory,
                QueryUserRequest(UserIndexOrName::Index(i)),
                None,
            )
            .await
            .unwrap()
            .accounts
            .pop_first()
            .unwrap();

        println!("user address {}: {}", i, user_address);
    }
}
