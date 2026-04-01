use {
    grug::{Addr, ContractWrapper, Message, addr},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-testnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("c4a8f7bbadd1457092a8cd182480230c0a848331");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/testnet-owner.json";

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        let code = ContractWrapper::from_index(13).to_bytes();

        Ok(Message::upload(code.to_vec()))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
