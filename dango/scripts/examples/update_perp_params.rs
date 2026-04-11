//! Currently, we have a vault deposit cap of $500k, and pair parameters assuming
//! actual deposit of $200k in the vault.
//!
//! The $500k deposit cap has been hit. We would like to:
//!
//! 1. Update deposit cap to $1M.
//! 2. Update pair parameters assuming $500k vault deposit (up from $200k):
//!
//!    | Pair | max_abs_oi   | vault_max_quote_size | impact_size     |
//!    |------|--------------|----------------------|-----------------|
//!    | BTC  | 8 -> 20      | 3 -> 8               | $60k -> $150k   |
//!    | ETH  | 200 -> 500   | 60 -> 150            | $35k -> $80k    |
//!    | SOL  | 3500 -> 9000 | 1000 -> 2500         | $20k -> $50k    |
//!    | HYPE | 4000 (same)  | 1500 (same)          | $10k -> $15k    |
//!
//! To run this script:
//!
//! ```bash
//! cargo run -p dango-scripts --example update_perp_params
//! ```

use {
    anyhow::{anyhow, ensure},
    dango_types::{
        Quantity, UsdValue,
        constants::{perp_btc, perp_eth, perp_hype, perp_sol},
        perps::{self, PairId, PairParam},
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
        // -------------------------- 1. Global param --------------------------

        // Query current global params and all pair params.
        let mut param = client
            .query_wasm_smart(PERPS_ADDRESS, perps::QueryParamRequest {}, None)
            .await?;

        // Vault deposit cap: $500k -> $1M
        {
            ensure!(
                param.vault_deposit_cap == Some(UsdValue::new_int(500_000)),
                "expecting current vault_deposit_cap to be $500k, found: {:?}",
                param.vault_deposit_cap
            );

            param.vault_deposit_cap = Some(UsdValue::new_int(1_000_000));
        }

        // -------------------------- 2. Pair params ---------------------------

        let mut pair_params = client
            .query_wasm_smart(
                PERPS_ADDRESS,
                perps::QueryPairParamsRequest {
                    start_after: None,
                    limit: None,
                },
                None,
            )
            .await?;

        // BTC: max_abs_oi 8 -> 20, vault_max_quote_size 3 -> 8, impact_size 60k -> 150k
        update_pair_param(
            &mut pair_params,
            &perp_btc::DENOM,
            Quantity::new_int(8),
            Quantity::new_int(20),
            Quantity::new_int(3),
            Quantity::new_int(8),
            UsdValue::new_int(60_000),
            UsdValue::new_int(150_000),
        )?;

        // ETH: max_abs_oi 200 -> 500, vault_max_quote_size 60 -> 150, impact_size 35k -> 80k
        update_pair_param(
            &mut pair_params,
            &perp_eth::DENOM,
            Quantity::new_int(200),
            Quantity::new_int(500),
            Quantity::new_int(60),
            Quantity::new_int(150),
            UsdValue::new_int(35_000),
            UsdValue::new_int(80_000),
        )?;

        // SOL: max_abs_oi 3500 -> 9000, vault_max_quote_size 1000 -> 2500, impact_size 20k -> 25k
        update_pair_param(
            &mut pair_params,
            &perp_sol::DENOM,
            Quantity::new_int(3_500),
            Quantity::new_int(9_000),
            Quantity::new_int(1_000),
            Quantity::new_int(2_500),
            UsdValue::new_int(20_000),
            UsdValue::new_int(25_000),
        )?;

        // HYPE: max_abs_oi unchanged (4000), vault_max_quote_size unchanged (1500), impact_size 10k -> 15k
        update_pair_param(
            &mut pair_params,
            &perp_hype::DENOM,
            Quantity::new_int(4_000),
            Quantity::new_int(4_000),
            Quantity::new_int(1_500),
            Quantity::new_int(1_500),
            UsdValue::new_int(10_000),
            UsdValue::new_int(15_000),
        )?;

        Ok(Message::execute(
            PERPS_ADDRESS,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure { param, pair_params }),
            Coins::new(),
        )?)
    }
}

fn update_pair_param(
    pair_params: &mut BTreeMap<PairId, PairParam>,
    pair_id: &PairId,
    old_max_abs_oi: Quantity,
    new_max_abs_oi: Quantity,
    old_vault_max_quote_size: Quantity,
    new_vault_max_quote_size: Quantity,
    old_impact_size: UsdValue,
    new_impact_size: UsdValue,
) -> anyhow::Result<()> {
    let p = pair_params
        .get_mut(pair_id)
        .ok_or_else(|| anyhow!("pair params not found for id {pair_id}"))?;

    {
        ensure!(
            p.max_abs_oi == old_max_abs_oi,
            "expecting current {pair_id} max_abs_oi to be {old_max_abs_oi}, found: {}",
            p.max_abs_oi
        );

        p.max_abs_oi = new_max_abs_oi;
    }

    {
        ensure!(
            p.vault_max_quote_size == old_vault_max_quote_size,
            "expecting current {pair_id} vault_max_quote_size to be {old_vault_max_quote_size}, found: {}",
            p.vault_max_quote_size
        );

        p.vault_max_quote_size = new_vault_max_quote_size;
    }

    {
        ensure!(
            p.impact_size == old_impact_size,
            "expecting current {pair_id} impact_size to be {old_impact_size}, found: {}",
            p.impact_size
        );

        p.impact_size = new_impact_size;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
