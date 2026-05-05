//! Subscribe to all `order_filled` events on Dango mainnet over WebSocket,
//! using the perps-specific `perpsTrades` subscription. A `block`
//! subscription runs alongside on the same multiplexed [`Session`] so we
//! also see a heartbeat for every finalized block.
//!
//! `perpsTrades` is pair-scoped, so we open one subscription per pair on
//! the shared session and merge the streams. Pair IDs are hardcoded here
//! for simplicity; production code should discover them via the
//! `allPerpsPairStats` query.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p dango-sdk --example subscribe_order_filled
//! ```

use {
    anyhow::Result,
    dango_sdk::{
        SubscribeBlock, SubscribePerpsTrades, WsClient, subscribe_block, subscribe_perps_trades,
    },
    futures::{StreamExt, stream::select_all},
};

const WS_URL: &str = "wss://api-mainnet.dango.zone/graphql";

const PAIR_IDS: &[&str] = &["perp/btcusd", "perp/ethusd"];

#[tokio::main]
async fn main() -> Result<()> {
    let session = WsClient::new(WS_URL)?.connect().await?;

    let mut blocks = session
        .subscribe::<SubscribeBlock>(subscribe_block::Variables {})
        .await?;

    let mut trades = {
        let mut trade_streams = Vec::with_capacity(PAIR_IDS.len());
        for pair_id in PAIR_IDS {
            let stream = session
                .subscribe::<SubscribePerpsTrades>(subscribe_perps_trades::Variables {
                    pair_id: (*pair_id).to_string(),
                })
                .await?;
            trade_streams.push(stream);
        }
        select_all(trade_streams)
    };

    println!("subscribed to block + perpsTrades for {PAIR_IDS:?} at {WS_URL}");

    loop {
        tokio::select! {
            item = blocks.next() => match item {
                Some(Ok(resp)) => {
                    if let Some(errors) = resp.errors {
                        for err in errors {
                            eprintln!("block graphql error: {err:?}");
                        }
                    }
                    if let Some(data) = resp.data {
                        println!("block={}", data.block.block_height);
                    }
                },
                Some(Err(err)) => eprintln!("block ws error: {err}"),
                None => break,
            },
            item = trades.next() => match item {
                Some(Ok(resp)) => {
                    if let Some(errors) = resp.errors {
                        for err in errors {
                            eprintln!("trade graphql error: {err:?}");
                        }
                    }
                    if let Some(data) = resp.data {
                        let trade = data.perps_trades;
                        println!(
                            "  fill block={} pair={} user={} size={} price={} fee={} fill_id={:?} maker={:?}",
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
                },
                Some(Err(err)) => eprintln!("trade ws error: {err}"),
                None => break,
            },
        }
    }

    Ok(())
}
