use {
    crate::{
        entity::{perps_events, perps_trade::PerpsTrade},
        error::Error,
    },
    dango_types::perps::OrderFilled,
    grug::{EventName, Timestamp},
    sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, Order, QueryFilter, QueryOrder},
    std::collections::HashMap,
};

#[derive(Debug, Default)]
pub struct PerpsTradeCache {
    trades: HashMap<String, Vec<PerpsTrade>>,
}

impl PerpsTradeCache {
    pub fn trades_for_pair(&self, pair_id: &str) -> Option<&Vec<PerpsTrade>> {
        self.trades.get(pair_id)
    }

    pub fn add_trades(&mut self, trades: Vec<PerpsTrade>) {
        for trade in trades {
            self.trades
                .entry(trade.pair_id.clone())
                .or_default()
                .push(trade);
        }
    }

    pub fn compact_keep_n(&mut self, n: usize) {
        for trades in self.trades.values_mut() {
            if trades.len() > n {
                trades.drain(0..trades.len() - n);
            }
        }
    }

    /// Preload recent `OrderFilled` events from the `perps_events` table.
    pub async fn preload(&mut self, db: &DatabaseConnection) -> Result<(), Error> {
        let rows = perps_events::Entity::find()
            .filter(perps_events::Column::EventType.eq(OrderFilled::EVENT_NAME))
            .order_by(perps_events::Column::BlockHeight, Order::Desc)
            .order_by(perps_events::Column::Idx, Order::Desc)
            .all(db)
            .await?;

        let mut trades: HashMap<String, Vec<PerpsTrade>> = HashMap::new();

        for (trade_idx, row) in rows.into_iter().enumerate() {
            let Ok(order_filled) = serde_json::value::to_value(&row.data)
                .and_then(serde_json::from_value::<OrderFilled>)
            else {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    block_height = row.block_height,
                    "Failed to deserialize perps OrderFilled from perps_events row"
                );
                continue;
            };

            let perps_trade = PerpsTrade {
                order_id: order_filled.order_id.to_string(),
                pair_id: order_filled.pair_id.to_string(),
                user: order_filled.user.to_string(),
                fill_price: order_filled.fill_price.to_string(),
                fill_size: order_filled.fill_size.to_string(),
                closing_size: order_filled.closing_size.to_string(),
                opening_size: order_filled.opening_size.to_string(),
                realized_pnl: order_filled.realized_pnl.to_string(),
                fee: order_filled.fee.to_string(),
                created_at: Timestamp::from(row.created_at).to_rfc3339_string(),
                block_height: row.block_height as u64,
                trade_idx: trade_idx as u32,
                fill_id: order_filled.fill_id.as_ref().map(ToString::to_string),
                is_maker: order_filled.is_maker,
            };

            trades
                .entry(perps_trade.pair_id.clone())
                .or_default()
                .push(perps_trade);
        }

        // Reverse each pair's trades so they are chronological (oldest first),
        // since the DB query returned newest first.
        for pair_trades in trades.values_mut() {
            pair_trades.reverse();
        }

        self.trades = trades;

        Ok(())
    }
}
