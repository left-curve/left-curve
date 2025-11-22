use dango_types::dex::{AmountOption, TimeInForce};

use {
    anyhow::Context,
    grug::{Inner, MultiplyFraction},
};

use {
    dango_types::dex::{Direction as DexDirection, PriceOption},
    grug::{Denom, NonZero, Number, Uint128},
};

use {
    anyhow::Result,
    dango_types::dex::CreateOrderRequest,
    grug::{Duration, IsZero, Timestamp, Udec128},
    std::{
        collections::{BTreeMap, BTreeSet},
        fs::File,
        io::{BufRead, BufReader},
        path::Path,
        str::FromStr,
    },
};

/// Configuration for converting Coinbase orders to DEX orders
#[derive(Clone)]
pub struct DexOrderConfig {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    /// Base asset amount scaling factor: multiply Coinbase size (in base asset) by this
    /// For example, if BTC has 8 decimals and we want to scale to base units,
    /// we'd use 100_000_000. This is because coinbase quotes are in whole BTC units,
    /// and the DEX uses smallest units.
    pub base_amount_scale: u128,
    /// Quote asset amount scaling factor: multiply quote amount by this
    /// For example, if USDC has 6 decimals, we'd use 1_000_000. This is because coinbase quotes
    /// are in whole USDC units, and the DEX uses smallest units.
    pub quote_amount_scale: u128,

    pub passive_orders_per_side: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrderDirection {
    Bid,
    Ask,
}

impl From<DexDirection> for OrderDirection {
    fn from(direction: DexDirection) -> Self {
        match direction {
            DexDirection::Bid => OrderDirection::Bid,
            DexDirection::Ask => OrderDirection::Ask,
        }
    }
}

impl From<OrderDirection> for DexDirection {
    fn from(direction: OrderDirection) -> Self {
        match direction {
            OrderDirection::Bid => DexDirection::Bid,
            OrderDirection::Ask => DexDirection::Ask,
        }
    }
}

/// Represents a single Pyth oracle price update
#[derive(Debug, Clone)]
pub struct PythPrice {
    pub timestamp: Timestamp,
    pub price: Udec128,
    pub confidence: Udec128,
    pub expo: i32,
}

pub(crate) fn to_dango_dex_order(
    price: Udec128,
    size: Udec128,
    direction: OrderDirection,
    config: &DexOrderConfig,
) -> Result<CreateOrderRequest> {
    // Only create orders for non-zero sizes
    anyhow::ensure!(!size.is_zero(), "Cannot create order with zero size");

    // DEX prices are in base units: quote base units per base base units.
    // To convert from human price (quote per base) to base units:
    // price_base = price_human * quote_amount_scale / base_amount_scale
    //
    // We use checked_from_ratio(numerator, denominator), so:
    // - numerator = price_human * quote_amount_scale (scaled to integer)
    // - denominator = base_amount_scale
    let scaled_price =
        Udec128::checked_from_ratio(config.quote_amount_scale, config.base_amount_scale)?
            .checked_mul(price)?;

    // Scale the amount based on order direction:
    // - Buy (Bid): amount is in quote asset (USDC) = size * price
    // - Sell (Ask): amount is in base asset (BTC) = size
    let scaled_amount = match direction {
        OrderDirection::Bid => {
            // For buy orders, calculate quote amount: size (BTC) * price (USD/BTC) = USD
            let base_amount = Uint128::new(config.base_amount_scale).checked_mul_dec(size)?;
            base_amount.checked_mul_dec(scaled_price)?
        },
        OrderDirection::Ask => {
            // For sell orders, amount is in base asset (BTC)
            Uint128::new(config.base_amount_scale).checked_mul_dec(size)?
        },
    };

    anyhow::ensure!(
        *scaled_amount.inner() > 0,
        "Order amount is zero after scaling"
    );

    // Create price (using checked_from_ratio)
    let price = NonZero::new(scaled_price.convert_precision()?)
        .context("Price is zero after conversion")?;

    // Create amount
    let amount = NonZero::new(scaled_amount).context("Amount is zero after conversion")?;

    Ok(CreateOrderRequest {
        base_denom: config.base_denom.clone(),
        quote_denom: config.quote_denom.clone(),
        price: PriceOption::Limit(price),
        amount: AmountOption::new(direction.into(), amount),
        time_in_force: TimeInForce::GoodTilCanceled,
    })
}

/// A batch of orders with the latest oracle price
#[derive(Debug, Clone)]
pub struct OrderBatch {
    pub creates: BTreeMap<u64, CreateOrderRequest>,
    pub cancels: BTreeSet<u64>,
    pub changes: BTreeMap<u64, CreateOrderRequest>,
    pub oracle_price: PythPrice,
    pub block_time: Timestamp,
}

/// Represents a single order event from Bitstamp CSV file
#[derive(Debug, Clone)]
pub struct BitstampOrderEvent {
    pub timestamp: Timestamp,
    pub event_type: OrderEventType,
    pub order_id: u64,
    pub price: Udec128,
    pub amount: Udec128,
    pub side: OrderDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderEventType {
    Created,
    Deleted,
    Changed,
}

impl BitstampOrderEvent {
    /// Parse a CSV line into a BitstampOrderEvent
    fn from_csv_line(line: &str) -> Result<Self, String> {
        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() < 8 {
            return Err(format!("Not enough columns: {}", parts.len()));
        }

        let timestamp_millis = parts[0]
            .trim()
            .parse::<u64>()
            .map_err(|e| format!("Invalid timestamp: {}", e))?;
        let timestamp = Timestamp::from_millis(timestamp_millis as u128);

        let event_type = match parts[2].trim() {
            "order_created" => OrderEventType::Created,
            "order_deleted" => OrderEventType::Deleted,
            "order_changed" => OrderEventType::Changed,
            other => return Err(format!("Invalid event_type: {}", other)),
        };

        let order_id = parts[3]
            .trim()
            .parse::<u64>()
            .map_err(|e| format!("Invalid order_id: {}", e))?;

        let price =
            Udec128::from_str(parts[4].trim()).map_err(|e| format!("Invalid price: {}", e))?;

        let amount =
            Udec128::from_str(parts[5].trim()).map_err(|e| format!("Invalid amount: {}", e))?;

        let side = match parts[6].trim() {
            "buy" => OrderDirection::Bid,
            "sell" => OrderDirection::Ask,
            other => return Err(format!("Invalid side: {}", other)),
        };

        Ok(Self {
            timestamp,
            event_type,
            order_id,
            price,
            amount,
            side,
        })
    }
}

/// Iterator that yields OrderBatch objects from the adapter
pub struct OrderBatchIterator<'a> {
    adapter: &'a mut BitstampDataAdapter,
    time_diff: Duration,
    max_oracle_staleness: Duration,
    /// Single counter for order IDs (incremented for both buy and sell)
    order_id_counter: u64,
}

/// Market data adapter that reads from CSV and batches orders by time
pub struct BitstampDataAdapter {
    order_events: Vec<BitstampOrderEvent>,
    prices: Vec<PythPrice>,
    current_order_index: usize,
    current_price_index: usize,
    current_timestamp: Timestamp,
    pub config: DexOrderConfig,
}

impl BitstampDataAdapter {
    /// Create a new adapter from order and price CSV file paths
    pub fn from_csv<P: AsRef<Path>>(
        orders_path: P,
        prices_path: P,
        config: DexOrderConfig,
    ) -> Result<Self, String> {
        // Load order events
        let order_events = Self::load_order_events(orders_path)?;

        // Load prices
        let prices = Self::load_prices(prices_path)?;

        let current_timestamp = std::cmp::min(
            order_events
                .first()
                .map(|o| o.timestamp)
                .unwrap_or_default(),
            prices.first().map(|p| p.timestamp).unwrap_or_default(),
        );

        println!(
            "Successfully loaded {} valid order events and {} price updates from CSV",
            order_events.len(),
            prices.len()
        );

        Ok(Self {
            order_events,
            prices,
            current_order_index: 0,
            current_price_index: 0,
            current_timestamp,
            config,
        })
    }

    /// Load order events from CSV file
    fn load_order_events<P: AsRef<Path>>(path: P) -> Result<Vec<BitstampOrderEvent>, String> {
        let file =
            File::open(path).map_err(|e| format!("Failed to open orders CSV file: {}", e))?;
        let reader = BufReader::new(file);

        let mut events = Vec::new();
        let mut is_first_line = true;

        for (line_num, line_result) in reader.lines().enumerate() {
            let line =
                line_result.map_err(|e| format!("Failed to read line {}: {}", line_num + 1, e))?;

            // Skip header
            if is_first_line {
                is_first_line = false;
                continue;
            }

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse the order event
            match BitstampOrderEvent::from_csv_line(&line) {
                Ok(event) => {
                    events.push(event);
                },
                Err(e) => {
                    // Log the error but continue processing
                    eprintln!(
                        "Warning: Skipping order line {} due to error: {}",
                        line_num + 1,
                        e
                    );
                },
            }
        }

        Ok(events)
    }

    /// Load prices from CSV file (Bitstamp format with integer prices)
    fn load_prices<P: AsRef<Path>>(path: P) -> Result<Vec<PythPrice>, String> {
        let file =
            File::open(path).map_err(|e| format!("Failed to open prices CSV file: {}", e))?;
        let reader = BufReader::new(file);

        let mut prices = Vec::new();
        let mut is_first_line = true;

        for (line_num, line_result) in reader.lines().enumerate() {
            let line =
                line_result.map_err(|e| format!("Failed to read line {}: {}", line_num + 1, e))?;

            // Skip header
            if is_first_line {
                is_first_line = false;
                continue;
            }

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse the price (Bitstamp format: integer price with exponent)
            match Self::parse_pyth_price_line(&line) {
                Ok(price) => {
                    prices.push(price);
                },
                Err(e) => {
                    // Log the error but continue processing
                    eprintln!(
                        "Warning: Skipping price line {} due to error: {}",
                        line_num + 1,
                        e
                    );
                },
            }
        }

        Ok(prices)
    }

    /// Parse a Pyth price line from Bitstamp CSV format
    /// Format: timestamp_millis,timestamp_iso,price,conf,expo,publish_time
    /// Price is an integer that needs to be converted using the exponent
    fn parse_pyth_price_line(line: &str) -> Result<PythPrice, String> {
        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() < 6 {
            return Err(format!("Not enough columns: {}", parts.len()));
        }

        let timestamp_millis = parts[0]
            .trim()
            .parse::<u64>()
            .map_err(|e| format!("Invalid timestamp: {}", e))?;
        let timestamp = Timestamp::from_millis(timestamp_millis as u128);

        // Parse price as integer
        let price_int = parts[2]
            .trim()
            .parse::<u128>()
            .map_err(|e| format!("Invalid price integer: {}", e))?;

        // Parse confidence as integer
        let conf_int = parts[3]
            .trim()
            .parse::<u128>()
            .map_err(|e| format!("Invalid confidence integer: {}", e))?;

        let expo = parts[4]
            .trim()
            .parse::<i32>()
            .map_err(|e| format!("Invalid expo: {}", e))?;

        // Convert integer price to decimal using exponent
        // price_human = price_int * 10^expo
        // For negative exponent, we need to insert decimal point
        let (price, confidence) = if expo < 0 {
            // Negative exponent: format as decimal string
            let expo_abs = (-expo) as usize;

            // Need to pad with zeros before decimal point
            (
                Udec128::checked_from_atomics(price_int, expo_abs as u32)
                    .map_err(|e| format!("Failed to parse price: {}", e))?,
                Udec128::checked_from_atomics(conf_int, expo_abs as u32)
                    .map_err(|e| format!("Failed to parse confidence: {}", e))?,
            )
        } else {
            panic!("Positive exponent not supported");
        };

        Ok(PythPrice {
            timestamp,
            price,
            confidence,
            expo,
        })
    }

    /// Returns an iterator that yields OrderBatch objects.
    /// The iteration ends when either the orders or prices CSV file is exhausted.
    pub fn batches(
        &mut self,
        time_diff: Duration,
        max_oracle_staleness: Duration,
        order_id_initial_value: u64,
    ) -> OrderBatchIterator<'_> {
        OrderBatchIterator {
            adapter: self,
            time_diff,
            max_oracle_staleness,
            order_id_counter: order_id_initial_value,
        }
    }

    /// Get the current timestamp
    pub fn current_timestamp(&self) -> Timestamp {
        self.current_timestamp
    }

    /// Get the total number of order events
    pub fn total_orders(&self) -> usize {
        self.order_events.len()
    }

    /// Get the number of processed order events
    pub fn processed_orders(&self) -> usize {
        self.current_order_index
    }

    pub fn first_oracle_price(&self) -> Option<PythPrice> {
        self.prices.first().cloned()
    }
}

impl Iterator for OrderBatchIterator<'_> {
    type Item = OrderBatch;

    fn next(&mut self) -> Option<Self::Item> {
        self.order_id_counter += self.adapter.config.passive_orders_per_side as u64 * 4;

        // Check if we've exhausted either order events or prices
        if self.adapter.current_order_index >= self.adapter.order_events.len()
            || self.adapter.current_price_index >= self.adapter.prices.len()
        {
            return None;
        }

        let proposed_new_timestamp = self.adapter.current_timestamp + self.time_diff;

        // Look ahead from the last price index to find the latest price that fits the criteria
        let mut latest_price_in_window: Option<&PythPrice> = None;
        let mut price_scan_index = self.adapter.current_price_index;

        // Find the latest price <= proposed_new_timestamp
        while price_scan_index < self.adapter.prices.len() {
            let price = &self.adapter.prices[price_scan_index];
            if price.timestamp <= proposed_new_timestamp {
                latest_price_in_window = Some(price);
                price_scan_index += 1;
            } else {
                break;
            }
        }

        // Determine the actual new_timestamp based on oracle price staleness
        let new_timestamp = if let Some(latest_price) = latest_price_in_window {
            // Check if the latest price is fresh enough:
            // price.timestamp >= proposed_new_timestamp - max_oracle_staleness
            if latest_price.timestamp >= proposed_new_timestamp - self.max_oracle_staleness {
                // Price is fresh enough, use the proposed timestamp
                proposed_new_timestamp
            } else {
                latest_price.timestamp + self.max_oracle_staleness - Duration::from_millis(1)
            }
        } else {
            panic!("No price found in the proposed window");
        };

        // Now collect all order events in the determined time window
        let mut order_events_in_window = Vec::new();
        while self.adapter.current_order_index < self.adapter.order_events.len() {
            let event = &self.adapter.order_events[self.adapter.current_order_index];

            if event.timestamp <= new_timestamp {
                order_events_in_window.push(event.clone());
                self.adapter.current_order_index += 1;
            } else {
                break;
            }
        }

        // Collect all prices in the determined time window and find the latest one
        let oracle_price = if let Some(price) = latest_price_in_window {
            price.clone()
        } else {
            panic!("No price found in the proposed window");
        };

        let block_time = new_timestamp - self.adapter.current_timestamp;

        self.adapter.current_timestamp = new_timestamp;

        // Convert order events to DEX orders
        let config = self.adapter.config.clone();
        let (created_orders, cancel_order_ids, changed_orders) = self
            .convert_events_to_orders(order_events_in_window, &config)
            .unwrap();

        Some(OrderBatch {
            creates: created_orders,
            cancels: cancel_order_ids,
            changes: changed_orders,
            oracle_price,
            block_time,
        })
    }
}

impl OrderBatchIterator<'_> {
    /// Convert order events to DEX orders with order ID tracking
    /// Returns (create_orders, cancel_order)
    fn convert_events_to_orders(
        &mut self,
        events: Vec<BitstampOrderEvent>,
        config: &DexOrderConfig,
    ) -> Result<(
        BTreeMap<u64, CreateOrderRequest>,
        BTreeSet<u64>,
        BTreeMap<u64, CreateOrderRequest>,
    )> {
        // Loop through all events and keep only the last updated event for each order ID
        let mut events_map = BTreeMap::new();
        for event in events {
            events_map.insert(event.order_id, event);
        }
        let events = events_map.values().cloned().collect::<Vec<_>>();

        let mut create_orders = BTreeMap::new();
        let mut cancel_order_ids = BTreeSet::new();
        let mut changed_orders = BTreeMap::new();

        for event in events {
            match event.event_type {
                OrderEventType::Created => {
                    if let Some(create_req) = self.handle_create(&event, config)? {
                        create_orders.insert(event.order_id, create_req);
                    }
                },
                OrderEventType::Deleted => {
                    cancel_order_ids.insert(event.order_id);
                },
                // We treat an updated order as a cancellation and a new creation
                OrderEventType::Changed => {
                    if let Some(create_req) = self.handle_create(&event, config)? {
                        changed_orders.insert(event.order_id, create_req);
                    }
                },
            }
        }

        Ok((create_orders, cancel_order_ids, changed_orders))
    }

    fn handle_create(
        &mut self,
        event: &BitstampOrderEvent,
        config: &DexOrderConfig,
    ) -> Result<Option<CreateOrderRequest>> {
        // Only create orders with non-zero amount
        if event.amount.is_non_zero() {
            let create_req = to_dango_dex_order(event.price, event.amount, event.side, config)?;

            return Ok(Some(create_req));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::constants::{dango, usdc},
        std::path::PathBuf,
    };

    #[test]
    fn test_adapter_first_batch() {
        // Get the paths to the CSV files
        let orders_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/dex/market_data/bitstamp/bitstamp_btcusd_20251118_132040.csv");
        let prices_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/dex/market_data/bitstamp/pyth_btcusd_20251118_132040.csv");

        // Create a config for testing
        let config = DexOrderConfig {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            base_amount_scale: 1_000_000,
            quote_amount_scale: 1_000_000,
            passive_orders_per_side: 0,
        };

        // Create the adapter
        let mut adapter =
            BitstampDataAdapter::from_csv(&orders_path, &prices_path, config).unwrap();

        // Assert initial timestamp
        assert_eq!(
            adapter.current_timestamp(),
            Timestamp::from_millis(1763468440823)
        );

        // Get the first batch
        // Block time is 1000ms, so next timestamp should be 1763468440823 + 1000 = 1763468441823
        // There's an oracle price at 1763468441365, which is 458ms before 1763468441823
        // With max_oracle_staleness of 500ms, this price is fresh enough
        // So the batch should use the price from 1763468441365 and advance to 1763468441823
        let mut batches =
            adapter.batches(Duration::from_millis(1000), Duration::from_millis(500), 1);
        let first_batch = batches.next().expect("Should have at least one batch");

        // Verify the adapter timestamp advanced correctly
        assert_eq!(
            batches.adapter.current_timestamp(),
            Timestamp::from_millis(1763468441823),
            "Adapter timestamp should advance by block_time (1000ms)"
        );

        // We should have no cancellations in the first batch. Since no orders have been
        // placed on the DEX yet.
        assert!(first_batch.cancels.is_empty());

        // Verify the oracle price used in the batch
        // The batch should use the price from 1763468441365 (latest price within the window)
        // Price: 9141901805127, conf: 3700707677, expo: -8
        let expected_price = Udec128::checked_from_atomics(9141901805127u128, 8)
            .expect("Failed to create expected price");
        let expected_confidence = Udec128::checked_from_atomics(3700707677u128, 8)
            .expect("Failed to create expected confidence");
        let expected_price_timestamp = Timestamp::from_millis(1763468441365);

        assert_eq!(
            first_batch.oracle_price.timestamp, expected_price_timestamp,
            "Batch should use the latest oracle price within the time window"
        );
        assert_eq!(first_batch.oracle_price.expo, -8);
        assert_eq!(
            first_batch.oracle_price.price, expected_price,
            "Price should match exactly"
        );
        assert_eq!(
            first_batch.oracle_price.confidence, expected_confidence,
            "Confidence should match exactly"
        );

        // Verify the first order in the batch
        // First order_created event: timestamp 1763468440823, order_id 1944098677456897,
        // price 91421, amount 0.02187852, side sell
        assert!(
            !first_batch.creates.is_empty(),
            "First batch should contain at least one order"
        );
    }
}
