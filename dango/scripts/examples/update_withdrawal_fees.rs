use {
    dango_types::{
        constants::{eth, usdc},
        gateway::{self, Remote, WithdrawalFee},
    },
    grug::{Addr, Coins, Message, Uint128, addr},
    hyperlane_types::{addr32, constants::ethereum},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const GATEWAY: Addr = addr!("c51e2cbe9636a90c86463ac3eb18fbee92b700d1");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        Ok(Message::execute(
            GATEWAY,
            &gateway::ExecuteMsg::SetWithdrawalFees(vec![
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: addr32!(
                            "000000000000000000000000d05909852ae07118857f9d071781671d12c0f36c"
                        ),
                    },
                    fee: Uint128::new(100_000), // 0.1 USDC
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: addr32!(
                            "0000000000000000000000009d259aa1ec7324c7433b89d2935b08c30f3154cb"
                        ),
                    },
                    fee: Uint128::new(50_000_000_000_000), // 0.00005 ETH ~= 0.15 USD
                },
            ]),
            Coins::new(),
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
