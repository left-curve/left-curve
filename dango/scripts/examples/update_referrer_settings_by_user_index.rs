use {
    dango_types::{Dimensionless, account_factory::UserIndex, perps},
    grug::{Addr, Coins, Message, NonEmpty, Op, addr},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const PERPS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

// referrer address --> commission rate
const REFERRERS: &[(UserIndex, Dimensionless)] = &[
    // Template: replace with actual data.
    (0, Dimensionless::new_percent(30)),
];

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        unreachable!("this builder only builds multiple messages");
    }

    async fn build_messages(_client: &HttpClient) -> anyhow::Result<NonEmpty<Vec<Message>>> {
        let msgs = REFERRERS
            .iter()
            .map(|(user, commission_rate)| {
                Message::execute(
                    PERPS,
                    &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetCommissionRateOverride {
                        user: *user,
                        commission_rate: Op::Insert(*commission_rate),
                    }),
                    Coins::new(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(NonEmpty::new(msgs)?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
