use {
    grug::{Addr, QueryClientExt, TendermintRpcClient, addr},
    indexer_client::HttpClient,
};

const BOT: Addr = addr!("bed1fa8569d5a66935dea5a179b77ac06067de32");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let http_client = HttpClient::new("http://api.testnet.ovh2.dango.zone/")?;
    let rpc_client = TendermintRpcClient::new("http://ovh2:36657")?;

    let balances_httpd = http_client.query_balances(BOT, None, None, None).await?;
    println!("bot balances httpd: {balances_httpd}");

    let balances_rpc = rpc_client.query_balances(BOT, None, None, None).await?;
    println!("bot balances rpc: {balances_rpc}");

    Ok(())
}
