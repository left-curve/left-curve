//! Subscribe to BTC perps events on Dango testnet over the WebSocket endpoint
//! (`GET /ws`, `perpsEvents` channel). The feed is filtered to the BTC pair and
//! the order-lifecycle / forced-exit event types (`order_persisted`,
//! `order_removed`, `order_resized`, `order_filled`, `liquidated`,
//! `deleveraged`), and arrives grouped per block.
//!
//! The `event_types` / `pair_ids` / `users` / `order_ids` / `client_order_ids`
//! filters AND together; clear any of them to widen the feed.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p dango-sdk --example subscribe_perps_events2
//! ```

use {
    anyhow::Result,
    dango_order_book::{OrderPersisted, OrderRemoved, OrderResized},
    dango_primitives::EventName,
    dango_sdk::HttpClient,
    dango_types::{
        constants::perp_btc,
        perps::{Deleveraged, Liquidated, OrderFilled},
    },
    futures::StreamExt,
};

const HTTP_URL: &str = "https://api-testnet.dango.zone";

#[tokio::main]
async fn main() -> Result<()> {
    let client = HttpClient::new(HTTP_URL)?;

    // Filters AND together, so only BTC events of the listed types stream.
    let mut events = client
        .subscribe_perps_events2(
            None,
            Some(vec![
                OrderPersisted::EVENT_NAME.to_string(),
                OrderRemoved::EVENT_NAME.to_string(),
                OrderResized::EVENT_NAME.to_string(),
                OrderFilled::EVENT_NAME.to_string(),
                Liquidated::EVENT_NAME.to_string(),
                Deleveraged::EVENT_NAME.to_string(),
            ]),
            Some(vec![perp_btc::DENOM.to_string()]),
            None,
            None,
            None,
        )
        .await?;

    println!("subscribed to perps events for BTC at {HTTP_URL}");

    while let Some(item) = events.next().await {
        match item {
            Ok(batch) => {
                for event in &batch.events {
                    println!(
                        "block={} idx={} type={} user={:?} pair={:?} order_id={:?} client_order_id={:?} data={}",
                        batch.block_height,
                        event.idx,
                        event.event_type,
                        event.user,
                        event.pair_id,
                        event.order_id,
                        event.client_order_id,
                        event.data,
                    );
                }
            },
            Err(err) => eprintln!("ws error: {err}"),
        }
    }

    Ok(())
}
