use {
    dango_types::{
        constants::{eth, eth_usdc, usdc},
        dex::{self, Geometric, PairParams, PairUpdate, PassiveLiquidity},
    },
    grug::{
        Addr, Bounded, Coins, Denom, Message, NumberConst, Part, Udec128, Uint128, addr, btree_set,
    },
    indexer_client::HttpClient,
    std::str::FromStr,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const DEX: Addr = addr!("da32476efe31e535207f0ad690d337a4ebf54a22");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        Ok(Message::execute(
            DEX,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_parts([
                        dex::NAMESPACE.clone(),
                        Part::from_str("pool")?,
                        eth::SUBDENOM.clone(),
                        usdc::SUBDENOM.clone(),
                    ])?,
                    pool_type: PassiveLiquidity::Geometric(Geometric {
                        spacing: Udec128::new_bps(1),
                        ratio: Bounded::new_unchecked(Udec128::ONE),
                        limit: 1,
                    }),
                    bucket_sizes: btree_set! {
                        eth_usdc::ONE_HUNDREDTH,
                        eth_usdc::ONE_TENTH,
                        eth_usdc::ONE,
                        eth_usdc::TEN,
                        eth_usdc::FIFTY,
                        eth_usdc::ONE_HUNDRED,
                    },
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(1)), /* 0.01% ~= $0.3 spread */
                    min_order_size_base: Uint128::new(300_000_000_000_000),     // 0.0003 ETH ~= $1
                    min_order_size_quote: Uint128::new(1_000_000),              // 1 USDC
                },
            }])),
            Coins::new(),
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}

// After running this script, use the following query message to ensure the pair
// was created successfully:
//
// ```json
// {
//     "wasmSmart": {
//         "contract": "0xda32476efe31e535207f0ad690d337a4ebf54a22",
//         "msg": {
//             "pairs": {}
//         }
//     }
// }
// ```
