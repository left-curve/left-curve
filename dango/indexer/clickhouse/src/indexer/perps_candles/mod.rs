use {
    crate::{
        context::Context,
        entities::perps_pair_price::PerpsPairPrice,
        error::{IndexerError, Result},
        indexer::Indexer,
    },
    chrono::{DateTime, Utc},
    dango_types::perps::OrderFilled,
    grug::{
        Addr, BlockAndBlockOutcomeWithHttpDetails, CommitmentStatus, EventName, EventStatus,
        EvtCron, FlatCommitmentStatus, FlatEvent, FlatEventInfo, FlatEventStatus, JsonDeExt,
        NaiveFlatten, SearchEvent, Udec128_6, Udec128_24,
    },
    std::collections::HashMap,
};

pub mod cache;
pub mod generator;

impl Indexer {
    pub(crate) async fn store_perps_candles(
        perps_addr: &Addr,
        ctx: &grug_app::IndexerContext,
        context: &Context,
    ) -> Result<()> {
        let block_and_block_outcome = ctx
            .get::<BlockAndBlockOutcomeWithHttpDetails>()
            .ok_or(IndexerError::missing_block_or_block_outcome())?;

        let created_at = DateTime::<Utc>::from_naive_utc_and_offset(
            block_and_block_outcome
                .block
                .info
                .timestamp
                .to_naive_date_time(),
            Utc,
        );

        let block_height = block_and_block_outcome.block.info.height;

        // Collect fills grouped by pair_id: (high, low, close, volume, volume_usd)
        let mut fills_by_pair: HashMap<String, PerpsPairPriceAccumulator> = HashMap::new();

        // Process tx_outcomes (user-submitted submit_order)
        for ((_tx, _tx_hash), tx_outcome) in block_and_block_outcome
            .block
            .txs
            .iter()
            .zip(block_and_block_outcome.block_outcome.tx_outcomes.iter())
        {
            if tx_outcome.result.is_err() {
                continue;
            }

            let flat = tx_outcome.events.clone().flat();

            for event in flat {
                if event.commitment_status != FlatCommitmentStatus::Committed {
                    continue;
                }

                let FlatEvent::ContractEvent(ref contract_event) = event.event else {
                    continue;
                };

                if contract_event.contract != *perps_addr {
                    continue;
                }

                if contract_event.ty == OrderFilled::EVENT_NAME {
                    process_order_filled(contract_event, &mut fills_by_pair)?;
                }
            }
        }

        // Process cron_outcomes (liquidations, book-matched orders)
        for outcome in block_and_block_outcome.block_outcome.cron_outcomes.clone() {
            let CommitmentStatus::Committed(EventStatus::Ok(EvtCron {
                guest_event: EventStatus::Ok(event),
                ..
            })) = outcome.cron_event
            else {
                continue;
            };

            if event.contract != perps_addr {
                continue;
            }

            for event in event.naive_flatten(FlatCommitmentStatus::Committed, FlatEventStatus::Ok) {
                let FlatEventInfo {
                    event: FlatEvent::ContractEvent(ref contract_event),
                    commitment_status: FlatCommitmentStatus::Committed,
                    event_status: FlatEventStatus::Ok,
                    ..
                } = event
                else {
                    continue;
                };

                if contract_event.ty == OrderFilled::EVENT_NAME {
                    process_order_filled(contract_event, &mut fills_by_pair)?;
                }
            }
        }

        // Convert accumulated fills to PerpsPairPrice entries
        let pair_prices: Vec<PerpsPairPrice> = fills_by_pair
            .into_values()
            .map(|acc| PerpsPairPrice {
                pair_id: acc.pair_id,
                high: acc.high,
                low: acc.low,
                close: acc.close,
                volume: acc.volume,
                volume_usd: acc.volume_usd,
                created_at,
                block_height,
            })
            .collect();

        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.perps_order_filled_events.total")
            .increment(pair_prices.len() as u64);

        let candle_generator = generator::PerpsCandleGenerator::new(context.clone());

        candle_generator
            .add_pair_prices(block_height, created_at, pair_prices)
            .await
    }
}

/// Accumulator for aggregating multiple fills per pair within a single block
struct PerpsPairPriceAccumulator {
    pair_id: String,
    high: Udec128_24,
    low: Udec128_24,
    close: Udec128_24,
    volume: Udec128_6,
    volume_usd: Udec128_6,
}

fn process_order_filled(
    contract_event: &grug_types::CheckedContractEvent,
    fills_by_pair: &mut HashMap<String, PerpsPairPriceAccumulator>,
) -> Result<()> {
    let order_filled = contract_event
        .data
        .clone()
        .deserialize_json::<OrderFilled>()?;

    let pair_id = order_filled.pair_id.to_string();

    // Convert fill_price (UsdPrice = Number<N1,P1,Z0> wrapping Dec128_6) to Udec128_24
    // fill_price.into_inner() -> Dec128_6 (Dec<i128, 6>)
    // Dec128_6.0 -> Int<i128>, .0 -> i128 (raw value at 6 decimal places)
    // Udec128_24::raw(Int::new(raw as u128)) scales from 6 to 24 decimals
    let fill_price_raw: i128 = (order_filled.fill_price.into_inner().0).0;
    let fill_price_u128 = fill_price_raw.unsigned_abs();
    // Scale from 6 decimals to 24 decimals: multiply by 10^18
    let fill_price_24 = Udec128_24::raw(grug::Int::new(fill_price_u128 * 10u128.pow(18)));

    // Convert fill_size (Quantity = Number<P1,Z0,Z0> wrapping Dec128_6) to Udec128_6
    let fill_size_raw: i128 = (order_filled.fill_size.into_inner().0).0;
    let fill_size_abs = fill_size_raw.unsigned_abs();
    let fill_volume = Udec128_6::raw(grug::Int::new(fill_size_abs));

    // volume_usd = abs(fill_size) * fill_price (at 6 decimal precision)
    // fill_size is Dec128_6, fill_price is Dec128_6, product raw = fill_size_raw * fill_price_raw / 10^6
    let volume_usd_raw = fill_size_abs.saturating_mul(fill_price_u128) / 10u128.pow(6);
    let fill_volume_usd = Udec128_6::raw(grug::Int::new(volume_usd_raw));

    let acc = fills_by_pair
        .entry(pair_id.clone())
        .or_insert_with(|| PerpsPairPriceAccumulator {
            pair_id,
            high: fill_price_24,
            low: fill_price_24,
            close: fill_price_24,
            volume: Udec128_6::raw(grug::Int::new(0)),
            volume_usd: Udec128_6::raw(grug::Int::new(0)),
        });

    // Update OHLC
    acc.high = acc.high.max(fill_price_24);
    acc.low = acc.low.min(fill_price_24);
    acc.close = fill_price_24; // Last fill in block determines close

    // Accumulate volumes
    acc.volume += fill_volume;
    acc.volume_usd += fill_volume_usd;

    Ok(())
}
