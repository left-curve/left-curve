//! Subscribe to all `order_filled` events on Dango mainnet over WebSocket,
//! using the perps-specific `perpsTrades` subscription.
//!
//! `perpsTrades` is pair-scoped, so we open one subscription per pair on a
//! shared multiplexed [`Session`] and merge the streams. Pair IDs are
//! hardcoded here for simplicity; production code should discover them via
//! the `allPerpsPairStats` query.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p dango-sdk --example subscribe_order_filled
//! ```

use {
    anyhow::Result,
    dango_sdk::{SubscribePerpsTrades, WsClient, subscribe_perps_trades},
    futures::{StreamExt, stream::select_all},
};

const WS_URL: &str = "wss://api-mainnet.dango.zone/graphql";

const PAIR_IDS: &[&str] = &["perp/btcusd", "perp/ethusd"];

#[tokio::main]
async fn main() -> Result<()> {
    let session = WsClient::new(WS_URL)?.connect().await?;

    let mut streams = Vec::with_capacity(PAIR_IDS.len());
    for pair_id in PAIR_IDS {
        let stream = session
            .subscribe::<SubscribePerpsTrades>(subscribe_perps_trades::Variables {
                pair_id: (*pair_id).to_string(),
            })
            .await?;
        streams.push(stream);
    }

    println!("subscribed to perpsTrades for {:?} at {WS_URL}", PAIR_IDS);

    let mut merged = select_all(streams);

    while let Some(item) = merged.next().await {
        let resp = match item {
            Ok(resp) => resp,
            Err(err) => {
                eprintln!("websocket error: {err}");
                continue;
            },
        };

        if let Some(errors) = resp.errors {
            for err in errors {
                eprintln!("graphql error: {err:?}");
            }
        }

        let Some(data) = resp.data else {
            continue;
        };

        let trade = data.perps_trades;
        println!(
            "block={} pair={} user={} size={} price={} fee={} fill_id={:?} maker={:?}",
            trade.block_height,
            trade.pair_id,
            trade.user,
            trade.fill_size,
            trade.fill_price,
            trade.fee,
            trade.fill_id,
            trade.is_maker,
        );
    }

    Ok(())
}
