use {
    dango_types::config::AppConfig,
    grug::{Addr, Message, QueryClientExt, addr},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-testnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("c4a8f7bbadd1457092a8cd182480230c0a848331");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/testnet-owner.json";

const PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(client: &HttpClient) -> anyhow::Result<Message> {
        let mut app_cfg = client.query_app_config::<AppConfig>(None).await?;
        app_cfg.addresses.perps = PERPS_ADDRESS;

        Ok(Message::configure(None, Some(app_cfg))?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
