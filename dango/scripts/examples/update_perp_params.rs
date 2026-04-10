//! Update perps pair parameters for $200k vault (up from $100k at instantiation).
//!
//! | Pair | max_abs_oi   | vault_max_quote_size | impact_size     |
//! |------|--------------|----------------------|-----------------|
//! | BTC  | 5 -> 8       | 2 -> 3               | $40k -> $60k    |
//! | ETH  | 100 -> 200   | 30 -> 60             | $25k -> $35k    |
//! | SOL  | 1500 -> 3500 | 500 -> 1000          | $15k -> $20k    |
//! | HYPE | 4000 (same)  | 1500 (same)          | $7.5k -> $10k   |

use {
    dango_types::{
        Quantity, UsdValue,
        constants::{perp_btc, perp_eth, perp_hype, perp_sol},
        perps,
    },
    grug::{Addr, Coins, Message, QueryClientExt, addr},
    indexer_client::HttpClient,
    std::collections::BTreeMap,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(client: &HttpClient) -> anyhow::Result<Message> {
        // Query current global params and all pair params.
        let param: perps::Param = client
            .query_wasm_smart(PERPS_ADDRESS, perps::QueryParamRequest {}, None)
            .await?;

        let mut pair_params: BTreeMap<perps::PairId, perps::PairParam> = client
            .query_wasm_smart(
                PERPS_ADDRESS,
                perps::QueryPairParamsRequest {
                    start_after: None,
                    limit: None,
                },
                None,
            )
            .await?;

        // BTC: max_abs_oi 5 -> 8, vault_max_quote_size 2 -> 3, impact_size 40k -> 60k
        {
            let p = pair_params
                .get_mut(&perp_btc::DENOM)
                .expect("BTC pair not found");
            p.max_abs_oi = Quantity::new_int(8);
            p.vault_max_quote_size = Quantity::new_int(3);
            p.impact_size = UsdValue::new_int(60_000);
        }

        // ETH: max_abs_oi 100 -> 200, vault_max_quote_size 30 -> 60, impact_size 25k -> 35k
        {
            let p = pair_params
                .get_mut(&perp_eth::DENOM)
                .expect("ETH pair not found");
            p.max_abs_oi = Quantity::new_int(200);
            p.vault_max_quote_size = Quantity::new_int(60);
            p.impact_size = UsdValue::new_int(35_000);
        }

        // SOL: max_abs_oi 1500 -> 3500, vault_max_quote_size 500 -> 1000, impact_size 15k -> 20k
        {
            let p = pair_params
                .get_mut(&perp_sol::DENOM)
                .expect("SOL pair not found");
            p.max_abs_oi = Quantity::new_int(3_500);
            p.vault_max_quote_size = Quantity::new_int(1_000);
            p.impact_size = UsdValue::new_int(20_000);
        }

        // HYPE: max_abs_oi unchanged (4000), vault_max_quote_size unchanged (1500), impact_size 7.5k -> 10k
        {
            let p = pair_params
                .get_mut(&perp_hype::DENOM)
                .expect("HYPE pair not found");
            p.impact_size = UsdValue::new_int(10_000);
        }

        Ok(Message::execute(
            PERPS_ADDRESS,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure { param, pair_params }),
            Coins::new(),
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
