use {
    anyhow::anyhow,
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
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("https://api-devnet.dango.zone")?;

    let app_config: AppConfig = client.query_app_config(None).await?;

    println!("{:#?}", app_config.addresses);

    let config = client.query_config(None).await?;
    println!("owner {}", config.owner);
    println!("bank {}", config.bank);

    // Query the user addresses
    for i in 1..10 {
        let mut response = client
            .query_wasm_smart(
                app_config.addresses.account_factory,
                QueryUserRequest(UserIndexOrName::Index(i)),
                None,
            )
            .await?;
        let Some((user_address, _)) = response.accounts.pop_first() else {
            return Err(anyhow!("no account found for user index {i}"));
        };

        println!("user address {}: {}", i, user_address);
    }

    Ok(())
}
