use {
    grug::{Addr, BorshSerExt, Query, QueryClient, addr},
    indexer_client::HttpClient,
    std::{fs, path::PathBuf},
};

const PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("https://api-testnet.dango.zone/")?;

    println!("Fetching all perps contract storage (this may take a while)...");

    let res = client
        .query_app(
            Query::wasm_scan(PERPS_ADDRESS, None, None, Some(u32::MAX)),
            None,
        )
        .await?
        .into_wasm_scan();

    println!("Fetched {} key-value pairs", res.len());

    let bytes = res.to_borsh_vec()?;

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("upgrade")
        .join("testdata")
        .join("perps_storage.borsh");

    fs::write(&out_path, &bytes)?;

    println!("Saved {} bytes to {}", bytes.len(), out_path.display());

    Ok(())
}
