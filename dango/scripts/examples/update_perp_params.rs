//! To run this script:
//!
//! ```bash
//! cargo run -p dango-scripts --example update_perp_params
//! ```

use {
    anyhow::{anyhow, ensure},
    dango_types::{
        Dimensionless, Quantity, UsdPrice,
        constants::{perp_btc, perp_eth, perp_hype, perp_sol},
        perps::{self, PairId, PairParam},
    },
    grug::{Addr, Coins, Duration, Message, QueryClientExt, addr},
    indexer_client::HttpClient,
    std::collections::BTreeMap,
};

// Mainnet
const API_URL: &str = "https://api-mainnet.dango.zone/";
const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");
const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";
const PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

// // Testnet
// const API_URL: &str = "https://api-testnet.dango.zone/";
// const OWNER_ADDRESS: Addr = addr!("c4a8f7bbadd1457092a8cd182480230c0a848331");
// const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/testnet-owner.json";
// const PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(client: &HttpClient) -> anyhow::Result<Message> {
        // -------------------------- 1. Global param --------------------------

        let mut param = client
            .query_wasm_smart(PERPS_ADDRESS, perps::QueryParamRequest {}, None)
            .await?;

        param.funding_period = Duration::from_minutes(15);

        // // Vault deposit cap: $500k -> $1M
        // {
        //     ensure!(
        //         param.vault_deposit_cap == Some(UsdValue::new_int(500_000)),
        //         "expecting current vault_deposit_cap to be $500k, found: {:?}",
        //         param.vault_deposit_cap
        //     );

        //     param.vault_deposit_cap = Some(UsdValue::new_int(1_000_000));
        // }

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

        update_pair_param(
            &mut pair_params,
            &perp_btc::DENOM,
            UsdPrice::new_int(1),
            UsdPrice::new_raw(100_000),
            Dimensionless::new_raw(5),
            Dimensionless::new_raw(30),
            Dimensionless::ZERO,
            Dimensionless::new_raw(500_000),
            Dimensionless::ZERO,
            Dimensionless::new_raw(300_000),
            Quantity::ZERO,
            Quantity::new_int(8),
        )?;

        update_pair_param(
            &mut pair_params,
            &perp_eth::DENOM,
            UsdPrice::new_raw(100_000),
            UsdPrice::new_raw(10_000),
            Dimensionless::new_raw(5),
            Dimensionless::new_raw(75),
            Dimensionless::ZERO,
            Dimensionless::new_raw(500_000),
            Dimensionless::ZERO,
            Dimensionless::new_raw(300_000),
            Quantity::ZERO,
            Quantity::new_int(150),
        )?;

        update_pair_param(
            &mut pair_params,
            &perp_sol::DENOM,
            UsdPrice::new_raw(10_000),
            UsdPrice::new_raw(1_000),
            Dimensionless::new_raw(5),
            Dimensionless::new_raw(65),
            Dimensionless::ZERO,
            Dimensionless::new_raw(500_000),
            Dimensionless::ZERO,
            Dimensionless::new_raw(300_000),
            Quantity::ZERO,
            Quantity::new_int(2_500),
        )?;

        update_pair_param(
            &mut pair_params,
            &perp_hype::DENOM,
            UsdPrice::new_raw(10_000),
            UsdPrice::new_raw(1_000),
            Dimensionless::new_raw(5),
            Dimensionless::new_raw(150),
            Dimensionless::ZERO,
            Dimensionless::new_raw(500_000),
            Dimensionless::ZERO,
            Dimensionless::new_raw(300_000),
            Quantity::ZERO,
            Quantity::new_int(1_500),
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
    old_tick_size: UsdPrice,
    new_tick_size: UsdPrice,
    old_vault_half_spread: Dimensionless,
    new_vault_half_spread: Dimensionless,
    old_vault_size_skew_factor: Dimensionless,
    new_vault_size_skew_factor: Dimensionless,
    old_vault_spread_skew_factor: Dimensionless,
    new_vault_spread_skew_factor: Dimensionless,
    old_vault_max_skew_size: Quantity,
    new_vault_max_skew_size: Quantity,
) -> anyhow::Result<()> {
    let p = pair_params
        .get_mut(pair_id)
        .ok_or_else(|| anyhow!("pair params not found for id {pair_id}"))?;

    {
        ensure!(
            p.tick_size == old_tick_size,
            "expecting current {pair_id} tick_size to be {old_tick_size}, found: {}",
            p.tick_size
        );

        p.tick_size = new_tick_size;
    }

    {
        ensure!(
            p.vault_half_spread == old_vault_half_spread,
            "expecting current {pair_id} vault_half_spread to be {old_vault_half_spread}, found: {}",
            p.vault_half_spread
        );

        p.vault_half_spread = new_vault_half_spread;
    }

    {
        ensure!(
            p.vault_size_skew_factor == old_vault_size_skew_factor,
            "expecting current {pair_id} vault_size_skew_factor to be {old_vault_size_skew_factor}, found: {}",
            p.vault_size_skew_factor
        );

        p.vault_size_skew_factor = new_vault_size_skew_factor;
    }

    {
        ensure!(
            p.vault_spread_skew_factor == old_vault_spread_skew_factor,
            "expecting current {pair_id} vault_spread_skew_factor to be {old_vault_spread_skew_factor}, found: {}",
            p.vault_spread_skew_factor
        );

        p.vault_spread_skew_factor = new_vault_spread_skew_factor;
    }

    {
        ensure!(
            p.vault_max_skew_size == old_vault_max_skew_size,
            "expecting current {pair_id} vault_size_skew_factor to be {old_vault_max_skew_size}, found: {}",
            p.vault_max_skew_size
        );

        p.vault_max_skew_size = new_vault_max_skew_size;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
