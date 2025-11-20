use {
    anyhow::{Context, Result},
    dango_types::dex::{
        AmountOption, CancelOrderRequest, CreateOrderRequest, Direction as DexDirection, OrderId,
        PriceOption, TimeInForce,
    },
    grug::{
        Denom, Duration, Inner, IsZero, MultiplyFraction, NonZero, Number, NumberConst, Timestamp,
        Udec128, Uint128,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        fs::File,
        io::{BufRead, BufReader},
        path::Path,
        str::FromStr,
    },
};

/// Represents the absolute orderbook depth at a specific price level from the Coinbase CSV file
/// Note: The size represents the ABSOLUTE quantity at this price level, not a delta
#[derive(Debug, Clone)]
pub struct CoinbaseOrderDepth {
    pub timestamp: Timestamp,
    pub side: OrderDirection,
    pub price: Udec128,
    pub size: Udec128,
}

/// Represents the final state at a price level after processing a batch
#[derive(Debug, Clone)]
pub struct PriceLevel {
    direction: OrderDirection,
    price: Udec128,
    size: Udec128, // Final absolute size at this price level
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

impl CoinbaseOrderDepth {
    /// Parse a CSV line into a CoinbaseOrder
    fn from_csv_line(line: &str) -> Result<Self, String> {
        let parts: Vec<&str> = line.split(',').collect();

        if parts.len() < 7 {
            return Err(format!("Not enough columns: {}", parts.len()));
        }

        let timestamp_millis = parts[0]
            .trim()
            .parse::<u64>()
            .map_err(|e| format!("Invalid timestamp: {}", e))?;
        let timestamp = Timestamp::from_millis(timestamp_millis as u128);

        let side = match parts[4].trim() {
            "buy" => OrderDirection::Bid,
            "sell" => OrderDirection::Ask,
            other => return Err(format!("Invalid side: {}", other)),
        };

        let price =
            Udec128::from_str(parts[5].trim()).map_err(|e| format!("Invalid price: {}", e))?;

        let size =
            Udec128::from_str(parts[6].trim()).map_err(|e| format!("Invalid size: {}", e))?;

        Ok(Self {
            timestamp,
            side,
            price,
            size,
        })
    }

    /// Check if this is a cancellation (size is zero)
    pub fn is_cancellation(&self) -> bool {
        self.size.is_zero()
    }
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

/// Represents a single Pyth oracle price update
#[derive(Debug, Clone)]
pub struct PythPrice {
    pub timestamp: Timestamp,
    pub price: Udec128,
    pub confidence: Udec128,
    pub expo: i32,
}

impl PythPrice {
    /// Parse a CSV line into a PythPrice
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

        let price = Udec128::from_str(parts[4]).map_err(|e| format!("Invalid price: {}", e))?;

        let confidence =
            Udec128::from_str(parts[5]).map_err(|e| format!("Invalid confidence: {}", e))?;

        let expo = parts[6]
            .trim()
            .parse::<i32>()
            .map_err(|e| format!("Invalid expo: {}", e))?;

        Ok(Self {
            timestamp,
            price,
            confidence,
            expo,
        })
    }
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

/// Iterator that yields OrderBatch objects from the adapter
pub struct OrderBatchIterator<'a> {
    adapter: &'a mut CoinbaseDataAdapter,
    time_diff: Duration,
    max_oracle_staleness: Duration,
    /// Single counter for order IDs (incremented for both buy and sell)
    order_id_counter: u64,
    /// Mapping from (side, price) to order_id
    price_level_to_order_id: BTreeMap<(Udec128, OrderDirection), OrderId>,
}

/// Market data adapter that reads from CSV and batches orders by time
pub struct CoinbaseDataAdapter {
    order_depths: Vec<CoinbaseOrderDepth>,
    prices: Vec<PythPrice>,
    current_order_index: usize,
    current_price_index: usize,
    current_timestamp: Timestamp,
    pub config: DexOrderConfig,
    /// Internal orderbook state: (Price, Side) -> Size
    /// Tracks the current absolute quantity at each price level
    orderbook_state: BTreeMap<(Udec128, OrderDirection), Udec128>,
}

impl CoinbaseDataAdapter {
    /// Create a new adapter from order and price CSV file paths
    pub fn from_csv<P: AsRef<Path>>(
        orders_path: P,
        prices_path: P,
        config: DexOrderConfig,
    ) -> Result<Self, String> {
        // Load order depths
        let order_depths = Self::load_order_depths(orders_path)?;

        // Load prices
        let prices = Self::load_prices(prices_path)?;

        let current_timestamp = std::cmp::min(
            order_depths
                .first()
                .map(|o| o.timestamp)
                .unwrap_or_default(),
            prices.first().map(|p| p.timestamp).unwrap_or_default(),
        );

        println!(
            "Successfully loaded {} valid order depths and {} price updates from CSV",
            order_depths.len(),
            prices.len()
        );

        let orderbook_state = BTreeMap::new();

        Ok(Self {
            order_depths,
            prices,
            current_order_index: 0,
            current_price_index: 0,
            current_timestamp,
            config,
            orderbook_state,
        })
    }

    /// Load order depths from CSV file
    fn load_order_depths<P: AsRef<Path>>(path: P) -> Result<Vec<CoinbaseOrderDepth>, String> {
        let file =
            File::open(path).map_err(|e| format!("Failed to open orders CSV file: {}", e))?;
        let reader = BufReader::new(file);

        let mut orders = Vec::new();
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

            // Parse the order depth
            match CoinbaseOrderDepth::from_csv_line(&line) {
                Ok(depth) => {
                    // Keep all order depths - we need to track orderbook state changes
                    // Size of 0 means the price level should be removed
                    orders.push(depth);
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

        Ok(orders)
    }

    /// Load prices from CSV file
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

            // Parse the price
            match PythPrice::from_csv_line(&line) {
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

    /// Returns an iterator that yields OrderBatch objects.
    /// The iteration ends when either the orders or prices CSV file is exhausted.
    pub fn batches<'a>(
        &'a mut self,
        time_diff: Duration,
        max_oracle_staleness: Duration,
    ) -> OrderBatchIterator<'a> {
        OrderBatchIterator {
            adapter: self,
            time_diff,
            max_oracle_staleness,
            order_id_counter: 1,
            price_level_to_order_id: BTreeMap::new(),
        }
    }
}

impl<'a> Iterator for OrderBatchIterator<'a> {
    type Item = OrderBatch;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if we've exhausted either order depths or prices
        if self.adapter.current_order_index >= self.adapter.order_depths.len()
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
                // Price is too stale. Check if it's in the window
                // (current_timestamp < price.timestamp < current_timestamp + time_diff_millis)
                if latest_price.timestamp > self.adapter.current_timestamp
                    && latest_price.timestamp < proposed_new_timestamp
                {
                    // Found a price in the window, set new_timestamp = price.timestamp + max_oracle_staleness - 1
                    latest_price.timestamp + self.max_oracle_staleness - Duration::from_millis(1)
                } else {
                    // No price found strictly in the window, use proposed timestamp
                    proposed_new_timestamp
                }
            }
        } else {
            // No price found in the proposed window, check if we've exhausted prices
            if self.adapter.current_price_index >= self.adapter.prices.len() {
                return None;
            }
            // Use proposed timestamp
            proposed_new_timestamp
        };

        // Now collect all order depths in the determined time window
        let mut order_depth_updates = Vec::new();
        while self.adapter.current_order_index < self.adapter.order_depths.len() {
            let depth = &self.adapter.order_depths[self.adapter.current_order_index];

            if depth.timestamp <= new_timestamp {
                order_depth_updates.push(depth.clone());
                self.adapter.current_order_index += 1;
            } else {
                break;
            }
        }

        // Collect all prices in the determined time window and find the latest one
        let mut latest_price: Option<PythPrice> = None;
        while self.adapter.current_price_index < self.adapter.prices.len() {
            let price = &self.adapter.prices[self.adapter.current_price_index];

            if price.timestamp <= new_timestamp {
                // Update latest price (prices are assumed to be in chronological order)
                latest_price = Some(price.clone());
                self.adapter.current_price_index += 1;
            } else {
                break;
            }
        }

        // If we didn't find a price, we've exhausted the price series
        let oracle_price = latest_price?;

        self.adapter.current_timestamp = new_timestamp;

        // Get the final updated price levels after processing a batch of order depth
        // updates.
        //
        // Every CoinbaseOrderDepth object contains the total available liquidity at a
        // specific price level and side. This returns the final state at each price level
        // that has been changed during the batch.
        let mut final_order_depth_updates: BTreeMap<(Udec128, OrderDirection), Udec128> =
            BTreeMap::new();
        for depth in order_depth_updates {
            final_order_depth_updates.insert((depth.price, depth.side), depth.size);
        }

        // Convert final price levels to DEX orders with order ID tracking
        // Clone config to avoid borrow checker issues
        // let config = self.adapter.config.clone();
        // let (created_orders, cancel_order_ids, changed_orders) = self
        //     .convert_final_levels_to_orders(final_order_depth_updates, &config)
        //     .unwrap();

        // Some(OrderBatch {
        //     creates: created_orders,
        //     cancels: cancel_order_ids,
        //     changes: changed_orders,
        //     oracle_price,
        // })
        todo!()
    }
}

impl<'a> OrderBatchIterator<'a> {
    /// Convert final price levels to DEX orders with order ID tracking
    /// Returns (create_orders, cancel_order)
    fn convert_final_levels_to_orders(
        &mut self,
        order_depth_updates: BTreeMap<(Udec128, OrderDirection), Udec128>,
        config: &DexOrderConfig,
    ) -> Result<(Vec<CreateOrderRequest>, Option<CancelOrderRequest>)> {
        let mut create_orders = Vec::new();
        let mut cancel_order_ids = BTreeSet::new();

        for ((price, side), size) in order_depth_updates {
            // Check if there's an existing order at this price level
            if let Some(existing_order_id) = self.price_level_to_order_id.get(&(price, side)) {
                // Cancel the existing order first (we'll replace it with new amount)
                cancel_order_ids.insert(*existing_order_id);
                self.price_level_to_order_id.remove(&(price, side));
            }

            // Create the new order with the final size if it's non-zero
            if size.is_non_zero() {
                let create_req = to_dango_dex_order(price, size, side, config)?;
                // Get the next order ID
                let raw_order_id = self.order_id_counter;
                self.order_id_counter += 1;

                // For buy orders, invert the order ID (count down from max)
                // For sell orders, use as-is (count up from 0)
                let order_id = match side {
                    OrderDirection::Bid => OrderId::new(!raw_order_id),
                    OrderDirection::Ask => OrderId::new(raw_order_id),
                };

                println!("Saving order with id: {order_id}, size: {size}");

                // Update the mapping
                self.price_level_to_order_id.insert((price, side), order_id);

                create_orders.push(create_req);
            }
        }

        let cancel_order = if cancel_order_ids.is_empty() {
            None
        } else {
            Some(CancelOrderRequest::Some(cancel_order_ids))
        };

        Ok((create_orders, cancel_order))
    }
}

impl CoinbaseDataAdapter {
    /// Get the raw Coinbase order depths in a time window (without creating transactions)
    /// Useful for testing or analysis
    pub fn peek_order_depths(&self, time_diff: Duration) -> Vec<CoinbaseOrderDepth> {
        let new_timestamp = self.current_timestamp + time_diff;
        let mut batch = Vec::new();
        let mut index = self.current_order_index;

        while index < self.order_depths.len() {
            let depth = &self.order_depths[index];

            if depth.timestamp <= new_timestamp {
                batch.push(depth.clone());
                index += 1;
            } else {
                break;
            }
        }

        batch
    }

    /// Get the current timestamp
    pub fn current_timestamp(&self) -> Timestamp {
        self.current_timestamp
    }

    /// Check if there are more order depths to process
    pub fn has_more_orders(&self) -> bool {
        self.current_order_index < self.order_depths.len()
    }

    /// Get the total number of order depths
    pub fn total_orders(&self) -> usize {
        self.order_depths.len()
    }

    /// Get the number of processed orders
    pub fn processed_orders(&self) -> usize {
        self.current_order_index
    }
}

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

#[cfg(test)]
mod tests {
    use {super::*, dango_types::dex::Price, test_case::test_case};

    // Note: Parse tests are kept as separate #[test] functions rather than using test_case
    // because they test different CSV formats (order depth, cancellation, price) with
    // different validation logic. Combining them would reduce clarity.

    #[test]
    fn test_parse_coinbase_order_depth() {
        let line =
            "1762954951637,2025-11-12T13:42:31.637290Z,l2update,BTC-USD,buy,105040.20,0.00123456";

        let depth = CoinbaseOrderDepth::from_csv_line(line).unwrap();
        assert_eq!(depth.timestamp, Timestamp::from_millis(1762954951637));
        assert_eq!(depth.side, OrderDirection::Bid);
        assert_eq!(depth.price, Udec128::from_str("105040.20").unwrap());
        assert_eq!(depth.size, Udec128::from_str("0.00123456").unwrap());
        assert!(!depth.is_cancellation());
    }

    #[test]
    fn test_parse_cancellation() {
        let line =
            "1762954951637,2025-11-12T13:42:31.637290Z,l2update,BTC-USD,sell,105109.02,0.00000000";

        let depth = CoinbaseOrderDepth::from_csv_line(line).unwrap();
        assert!(depth.is_cancellation());
    }

    #[test]
    fn test_parse_pyth_price() {
        let line = "1763040722687,2025-11-13T13:32:02.687390+00:00,e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43,BTC/USD,102578.20982595,36.12254814,-8,1763040721";

        let price = PythPrice::from_csv_line(line).unwrap();
        assert_eq!(price.timestamp, Timestamp::from_millis(1763040722687));
        assert_eq!(price.price, Udec128::from_str("102578.20982595").unwrap());
        assert_eq!(price.confidence, Udec128::from_str("36.12254814").unwrap());
        assert_eq!(price.expo, -8);
    }

    // Using test_case to test both Bid and Ask directions with the same logic,
    // reducing code duplication while maintaining clear test cases for each direction.

    #[test_case(
        OrderDirection::Bid,
        "0.1",
        "100000.0",
        100_000_000,
        1_000_000,
        10_000_000_000_u128,
        Price::from_str("1000.0").unwrap()
    )]
    #[test_case(
        OrderDirection::Ask,
        "0.5",
        "100000.0",
        100_000_000,
        1_000_000,
        50_000_000_u128,
        Price::from_str("1000.0").unwrap()
    )]
    #[test_case(
        OrderDirection::Bid,
        "0.0",
        "100000.0",
        100_000_000,
        1_000_000,
        0_u128,
        Price::from_str("1000.0").unwrap()
        => panics "Cannot create order with zero size"
    )]
    #[test_case(
        OrderDirection::Ask, 
        "0.0", 
        "100000.0", 
        100_000_000,
        1_000_000, 
        0_u128, 
        Price::from_str("1000.0").unwrap() 
        => panics "Cannot create order with zero size")]
    #[test_case(
        OrderDirection::Bid,
        "0.25",
        "80000.0",
        100_000_000,
        1_000_000,
        20_000_000_000_u128,
        Price::from_str("800.0").unwrap()  
    )]
    #[test_case(
        OrderDirection::Ask,
        "0.75",
        "90000.0",
        100_000_000,
        1_000_000,
        75_000_000_u128,
        Price::from_str("900.0").unwrap()
    )]
    fn test_to_dango_dex_order_basic(
        direction: OrderDirection,
        size_str: &str,
        price_str: &str,
        base_amount_scale: u128,
        quote_amount_scale: u128,
        expected_amount_value: u128,
        expected_price: Price,
    ) {
        let config = DexOrderConfig {
            base_denom: "btc".try_into().unwrap(),
            quote_denom: "usdc".try_into().unwrap(),
            base_amount_scale,
            quote_amount_scale,
            passive_orders_per_side: 0,
        };

        let price = Udec128::from_str(price_str).unwrap(); // $100k per BTC
        let size = Udec128::from_str(size_str).unwrap();

        let order = to_dango_dex_order(price, size, direction, &config).unwrap();

        let expected_amount = Uint128::new(expected_amount_value);
        match (&order.amount, direction) {
            (AmountOption::Bid { quote }, OrderDirection::Bid) => {
                assert_eq!(*quote.inner(), expected_amount);
            },
            (AmountOption::Ask { base }, OrderDirection::Ask) => {
                assert_eq!(*base.inner(), expected_amount);
            },
            _ => panic!("Amount type doesn't match direction"),
        }
        assert_eq!(order.direction(), direction.into());

        // Price should be scaled: 100000 * 1_000_000 / 100_000_000 = 1000
        // (quote base units per base base units)
        let PriceOption::Limit(price) = &order.price else {
            panic!("Expected Limit price");
        };
        assert_eq!(price.into_inner(), expected_price);

        assert_eq!(order.base_denom, config.base_denom);
        assert_eq!(order.quote_denom, config.quote_denom);
    }

    #[test]
    fn test_to_dango_dex_order_price_scaling() {
        let config = DexOrderConfig {
            base_denom: "btc".try_into().unwrap(),
            quote_denom: "usdc".try_into().unwrap(),
            base_amount_scale: 100_000_000, // 8 decimals
            quote_amount_scale: 1_000_000,  // 6 decimals
            passive_orders_per_side: 0,
        };

        // Test with a specific price to verify scaling
        let price = Udec128::from_str("50000.0").unwrap(); // $50k per BTC
        let size = Udec128::from_str("1.0").unwrap();

        let order = to_dango_dex_order(price, size, OrderDirection::Bid, &config).unwrap();

        // Price scaling: 50000 * 1_000_000 / 100_000_000 = 500
        // This means 500 quote base units per base base unit
        let expected_price = NonZero::new(
            Udec128::from_str("500.0")
                .unwrap()
                .convert_precision()
                .unwrap(),
        )
        .unwrap();
        let PriceOption::Limit(price) = &order.price else {
            panic!("Expected Limit price");
        };
        assert_eq!(*price.inner(), *expected_price.inner());
    }
}
