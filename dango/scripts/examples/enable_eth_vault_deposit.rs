use {
    dango_client::{Secp256k1, Secret, SingleSigner},
    dango_testing::constants::owner,
    dango_types::{
        Dimensionless, Quantity,
        config::AppConfig,
        constants::perp_eth,
        perps::{self, PairId, PairParam},
    },
    grug::{BroadcastClientExt, Coins, GasOption, JsonSerExt, Message, QueryClientExt, SearchTxClient},
    grug_app::GAS_COSTS,
    indexer_client::HttpClient,
    std::collections::BTreeMap,
};

const API_URL: &str = "https://api-devnet.dango.zone/";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new(API_URL)?;

    let status = client.query_status(None).await?;
    println!("Connected to devnet. Height: {}", status.last_finalized_block.height);

    let config = client.query_config(None).await?;
    let owner_address = config.owner;
    println!("Owner: {owner_address}");

    let app_cfg: AppConfig = client.query_app_config(None).await?;
    let perps_addr = app_cfg.addresses.perps;

    // Query current global params and all pair params.
    let mut param: perps::Param = client
        .query_wasm_smart(perps_addr, perps::QueryParamRequest {}, None)
        .await?;

    let mut pair_params: BTreeMap<PairId, PairParam> = client
        .query_wasm_smart(
            perps_addr,
            perps::QueryPairParamsRequest {
                start_after: None,
                limit: None,
            },
            None,
        )
        .await?;

    // Enable vault deposits for ETH-USD.
    let eth_pair = pair_params
        .get_mut(&perp_eth::DENOM)
        .expect("ETH-USD pair not found in perps contract");

    eth_pair.vault_half_spread = Dimensionless::new_raw(600); // 0.06% (6 bps)
    eth_pair.vault_liquidity_weight = Dimensionless::new_int(3);
    eth_pair.vault_max_quote_size = Quantity::new_int(2_000); // 2,000 ETH

    // Recalculate vault_total_weight as the sum of all pair weights.
    param.vault_total_weight = pair_params
        .values()
        .map(|p| p.vault_liquidity_weight)
        .try_fold(Dimensionless::ZERO, |acc, w| acc.checked_add(w))?;

    println!("vault_total_weight = {}", param.vault_total_weight);

    let msg = Message::execute(
        perps_addr,
        &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
            param,
            pair_params,
        }),
        Coins::new(),
    )?;

    let secret = Secp256k1::from_bytes(owner::PRIVATE_KEY)?;
    let mut signer = SingleSigner::new(owner_address, secret)
        .with_query_user_index(&client)
        .await?
        .with_query_nonce(&client)
        .await?;

    let outcome = client
        .send_message_with_confirmation(
            &mut signer,
            msg,
            GasOption::Simulate {
                scale: 2.,
                flat_increase: GAS_COSTS.secp256k1_verify,
            },
            &status.chain_id,
            |tx| {
                println!("{}", tx.to_json_string_pretty()?);
                Ok(dialoguer::Confirm::new()
                    .with_prompt("Broadcast transaction?")
                    .interact()?)
            },
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("User rejected transaction"))?;

    println!("Tx broadcasted: {}", outcome.tx_hash);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let outcome = client.search_tx(outcome.tx_hash).await?;
    println!("{}", outcome.to_json_string_pretty()?);

    Ok(())
}
