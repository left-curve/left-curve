use {
    anyhow::anyhow,
    dango_types::{Dimensionless, account_factory, perps},
    grug::{Addr, Coins, Message, NonEmpty, Op, QueryClientExt, addr},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const ACCOUNT_FACTORY: Addr = addr!("18d28bafcdf9d4574f920ea004dea2d13ec16f6b");

const PERPS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

// referrer address --> commission rate
const REFERRERS: &[(Addr, Dimensionless)] = &[
    // Template: replace with actual data.
    (
        addr!("0000000000000000000000000000000000000000"),
        Dimensionless::new_percent(0),
    ),
];

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        unreachable!("this builder only builds multiple messages");
    }

    async fn build_messages(client: &HttpClient) -> anyhow::Result<NonEmpty<Vec<Message>>> {
        let msgs = futures::future::try_join_all(REFERRERS.iter().map(
            async |(address, commission_rate)| -> anyhow::Result<_> {
                let user = client
                    .query_wasm_smart(
                        ACCOUNT_FACTORY,
                        account_factory::QueryAccountRequest { address: *address },
                        None,
                    )
                    .await
                    .map_err(|err| {
                        anyhow!("failed to find user index for address {address}: {err}")
                    })?
                    .owner;

                let current_commission_rate_override = client
                    .query_wasm_smart(
                        PERPS,
                        perps::QueryCommissionRateOverrideRequest { user },
                        None,
                    )
                    .await
                    .map_err(|err| {
                        anyhow!("failed to find the current commission rate override of user {user}: {err}")
                    })?;

                println!(
                    "user: {user}, address: {address}, current commission rate: {}, setting to: {commission_rate}",
                    if let Some(rate) = current_commission_rate_override {
                        format!("Some({rate})")
                    } else {
                        "None".to_string()
                    }
                );

                Ok(Message::execute(
                    PERPS,
                    &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetCommissionRateOverride {
                        user,
                        commission_rate: Op::Insert(*commission_rate),
                    }),
                    Coins::new(),
                )?)
            },
        ))
        .await?;

        Ok(NonEmpty::new(msgs)?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
