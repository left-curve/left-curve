use {
    dango_types::{
        config::AppConfig,
        dex::{self, Direction, QueryOrdersByPairRequest},
    },
    grug::{Number, NumberConst, QueryClientExt, Udec128_6},
    indexer_client::HttpClient,
};

/// This script compares the reserves of Dango DEX pools with the sum of orders in those pools
/// and print the missing amounts if any discrepancies are found.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("https://testnet.dango.exchange");
    let cfg: AppConfig = client.query_app_config(None).await?;

    // Retrieve the pools reserves.
    let pools = client
        .query_wasm_smart(
            cfg.addresses.dex,
            dex::QueryReservesRequest {
                start_after: None,
                limit: None,
            },
            None,
        )
        .await?;

    for pool in pools {
        // Retrieve the orders for the current pool.
        let orders = client
            .query_wasm_smart(
                cfg.addresses.dex,
                QueryOrdersByPairRequest {
                    base_denom: pool.pair.base_denom.clone(),
                    quote_denom: pool.pair.quote_denom.clone(),
                    start_after: None,
                    limit: Some(u32::MAX),
                },
                None,
            )
            .await?;

        let (sum_base, sum_quote) = orders.values().fold(
            (Udec128_6::ZERO, Udec128_6::ZERO),
            |(mut sum_base, mut sum_quote), order| {
                if order.direction == Direction::Ask {
                    sum_base += order.remaining;
                } else {
                    sum_quote += order.remaining.checked_mul(order.price).unwrap();
                }

                (sum_base, sum_quote)
            },
        );

        let (pool_base, pool_quote) =
            match pool.reserve.first().denom.clone() == pool.pair.base_denom {
                true => (pool.reserve.first(), pool.reserve.second()),
                false => (pool.reserve.second(), pool.reserve.first()),
            };

        // BASE
        if *pool_base.amount < sum_base.into_int() {
            println!(
                "Warning: Pool {} {} base amount is less than the sum of orders.",
                pool.pair.base_denom, pool.pair.quote_denom
            );
            println!(
                "Base -> pool: {}, orders: {}, missing: {}",
                pool_base.amount,
                sum_base.into_int(),
                sum_base.into_int() - *pool_base.amount
            );
            println!();
        }

        // QUOTE
        if *pool_quote.amount < sum_quote.into_int() {
            println!(
                "Warning: Pool {} {} quote amount is less than the sum of orders.",
                pool.pair.base_denom, pool.pair.quote_denom
            );
            println!(
                "Quote -> pool: {}, orders: {}, missing: {}",
                pool_quote.amount,
                sum_quote.into_int(),
                sum_quote.into_int() - *pool_quote.amount
            );
            println!();
        }
    }

    Ok(())
}
