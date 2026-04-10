use {
    dango_types::{Dimensionless, perps},
    grug::{Addr, Coins, Message, addr},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const PERPS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        Ok(Message::execute(
            PERPS,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::ForceSetFeeShareRatio {
                // Replace with actual data
                user: 0,
                share_ratio: Dimensionless::new_percent(0),
            }),
            Coins::new(),
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
