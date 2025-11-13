use {
    anyhow::{Context, Result},
    dango_types::dex::{AmountOption, CreateOrderRequest, Direction, PriceOption, TimeInForce},
    grug::{Denom, NonZero, Uint128},
    std::{
        fs::File,
        io::{BufRead, BufReader},
        path::Path,
    },
};

/// Represents a single order from the Coinbase CSV file
#[derive(Debug, Clone)]
pub struct CoinbaseOrder {
    pub timestamp_millis: u64,
    pub side: Side,
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl CoinbaseOrder {
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

        let side = match parts[4].trim() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            other => return Err(format!("Invalid side: {}", other)),
        };

        let price = parts[5]
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("Invalid price: {}", e))?;

        let size = parts[6]
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("Invalid size: {}", e))?;

        Ok(Self {
            timestamp_millis,
            side,
            price,
            size,
        })
    }

    /// Check if this is a cancellation (size is zero)
    pub fn is_cancellation(&self) -> bool {
        self.size == 0.0
    }

    /// Convert this Coinbase order to a Dango DEX CreateOrderRequest
    pub fn to_dango_dex_order(&self, config: &DexOrderConfig) -> Result<CreateOrderRequest> {
        // Scale the price to DEX format
        let scaled_price = (self.price * config.price_scale as f64) as u128;
        anyhow::ensure!(scaled_price > 0, "Order price is zero");

        let direction = match self.side {
            Side::Buy => Direction::Bid,
            Side::Sell => Direction::Ask,
        };

        // Scale the amount based on order direction:
        // - Buy (Bid): amount is in quote asset (USDC) = size * price
        // - Sell (Ask): amount is in base asset (BTC) = size
        let scaled_amount = match direction {
            Direction::Bid => {
                // For buy orders, calculate quote amount: size (BTC) * price (USD/BTC) = USD
                let quote_amount = self.size * self.price;
                (quote_amount * config.quote_amount_scale as f64) as u128
            },
            Direction::Ask => {
                // For sell orders, amount is in base asset (BTC)
                (self.size * config.base_amount_scale as f64) as u128
            },
        };

        anyhow::ensure!(scaled_amount > 0, "Order amount is zero after scaling");

        // Create price (using checked_from_ratio)
        let price_dec =
            dango_types::dex::Price::checked_from_ratio(scaled_price, config.price_scale)
                .context("Failed to create price from ratio")?;
        let price = NonZero::new(price_dec).context("Price is zero after conversion")?;

        // Create amount
        let amount =
            NonZero::new(Uint128::new(scaled_amount)).context("Amount is zero after conversion")?;

        Ok(CreateOrderRequest {
            base_denom: config.base_denom.clone(),
            quote_denom: config.quote_denom.clone(),
            price: PriceOption::Limit(price),
            amount: AmountOption::new(direction, amount),
            time_in_force: TimeInForce::GoodTilCanceled,
        })
    }
}

/// Represents a single Pyth oracle price update
#[derive(Debug, Clone)]
pub struct PythPrice {
    pub timestamp_millis: u64,
    pub price: f64,
    pub confidence: f64,
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

        let price = parts[4]
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("Invalid price: {}", e))?;

        let confidence = parts[5]
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("Invalid confidence: {}", e))?;

        let expo = parts[6]
            .trim()
            .parse::<i32>()
            .map_err(|e| format!("Invalid expo: {}", e))?;

        Ok(Self {
            timestamp_millis,
            price,
            confidence,
            expo,
        })
    }
}

/// A batch of orders with the latest oracle price
#[derive(Debug, Clone)]
pub struct OrderBatch {
    pub orders: Vec<CreateOrderRequest>,
    pub latest_price: Option<PythPrice>,
}

/// Market data adapter that reads from CSV and batches orders by time
pub struct CoinbaseDataAdapter {
    orders: Vec<CoinbaseOrder>,
    prices: Vec<PythPrice>,
    current_order_index: usize,
    current_price_index: usize,
    current_timestamp: u64,
}

impl CoinbaseDataAdapter {
    /// Create a new adapter from order and price CSV file paths
    pub fn from_csv<P: AsRef<Path>>(orders_path: P, prices_path: P) -> Result<Self, String> {
        // Load orders
        let orders = Self::load_orders(orders_path)?;

        // Load prices
        let prices = Self::load_prices(prices_path)?;

        let current_timestamp = std::cmp::min(
            orders.first().map(|o| o.timestamp_millis).unwrap_or(0),
            prices.first().map(|p| p.timestamp_millis).unwrap_or(0),
        );

        println!(
            "Successfully loaded {} valid orders and {} price updates from CSV",
            orders.len(),
            prices.len()
        );

        Ok(Self {
            orders,
            prices,
            current_order_index: 0,
            current_price_index: 0,
            current_timestamp,
        })
    }

    /// Load orders from CSV file
    fn load_orders<P: AsRef<Path>>(path: P) -> Result<Vec<CoinbaseOrder>, String> {
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

            // Parse the order
            match CoinbaseOrder::from_csv_line(&line) {
                Ok(order) => {
                    // Skip cancellations for now
                    if !order.is_cancellation() {
                        orders.push(order);
                    }
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

    /// Advance time by the given number of milliseconds and return DEX order requests
    /// for all orders that occurred between the previous timestamp and the new timestamp.
    ///
    /// Orders that fail conversion (too small, invalid price, etc.) are skipped.
    /// Also returns the latest available oracle price within the batch time window.
    pub fn advance(&mut self, time_diff_millis: u64, config: &DexOrderConfig) -> OrderBatch {
        let new_timestamp = self.current_timestamp + time_diff_millis;
        let mut coinbase_orders = Vec::new();

        // Collect all orders in the time window
        while self.current_order_index < self.orders.len() {
            let order = &self.orders[self.current_order_index];

            if order.timestamp_millis <= new_timestamp {
                coinbase_orders.push(order.clone());
                self.current_order_index += 1;
            } else {
                break;
            }
        }

        // Collect all prices in the time window and find the latest one
        let mut latest_price: Option<PythPrice> = None;
        while self.current_price_index < self.prices.len() {
            let price = &self.prices[self.current_price_index];

            if price.timestamp_millis <= new_timestamp {
                // Update latest price (prices are assumed to be in chronological order)
                latest_price = Some(price.clone());
                self.current_price_index += 1;
            } else {
                break;
            }
        }

        self.current_timestamp = new_timestamp;

        // Convert to DEX orders, skipping any that fail
        let orders = batch_to_dex_orders(&coinbase_orders, config);

        OrderBatch {
            orders,
            latest_price,
        }
    }

    /// Get the raw Coinbase orders in a time window (without creating transactions)
    /// Useful for testing or analysis
    pub fn peek_orders(&self, time_diff_millis: u64) -> Vec<CoinbaseOrder> {
        let new_timestamp = self.current_timestamp + time_diff_millis;
        let mut batch = Vec::new();
        let mut index = self.current_order_index;

        while index < self.orders.len() {
            let order = &self.orders[index];

            if order.timestamp_millis <= new_timestamp {
                batch.push(order.clone());
                index += 1;
            } else {
                break;
            }
        }

        batch
    }

    /// Get the current timestamp
    pub fn current_timestamp(&self) -> u64 {
        self.current_timestamp
    }

    /// Check if there are more orders to process
    pub fn has_more_orders(&self) -> bool {
        self.current_order_index < self.orders.len()
    }

    /// Get the total number of orders
    pub fn total_orders(&self) -> usize {
        self.orders.len()
    }

    /// Get the number of processed orders
    pub fn processed_orders(&self) -> usize {
        self.current_order_index
    }
}

/// Configuration for converting Coinbase orders to DEX orders
pub struct DexOrderConfig {
    pub base_denom: Denom,
    pub quote_denom: Denom,
    /// Price scaling factor: multiply Coinbase price by this to get DEX price
    /// For example, if BTC is $100,000 and we want to represent it with 6 decimals,
    /// we'd use 1_000_000
    pub price_scale: u128,
    /// Base asset amount scaling factor: multiply Coinbase size (in base asset) by this
    /// For example, if BTC has 8 decimals and we want to scale to base units,
    /// we'd use 100_000_000
    pub base_amount_scale: u128,
    /// Quote asset amount scaling factor: multiply quote amount by this
    /// For example, if USDC has 6 decimals, we'd use 1_000_000
    pub quote_amount_scale: u128,
}

/// Helper function to convert a batch of Coinbase orders to DEX orders.
/// Orders that fail conversion are skipped.
pub fn batch_to_dex_orders(
    orders: &[CoinbaseOrder],
    config: &DexOrderConfig,
) -> Vec<CreateOrderRequest> {
    orders
        .iter()
        .filter_map(|order| order.to_dango_dex_order(config).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coinbase_order() {
        let line =
            "1762954951637,2025-11-12T13:42:31.637290Z,l2update,BTC-USD,buy,105040.20,0.00123456";

        let order = CoinbaseOrder::from_csv_line(line).unwrap();
        assert_eq!(order.timestamp_millis, 1762954951637);
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.price, 105040.20);
        assert_eq!(order.size, 0.00123456);
        assert!(!order.is_cancellation());
    }

    #[test]
    fn test_parse_cancellation() {
        let line =
            "1762954951637,2025-11-12T13:42:31.637290Z,l2update,BTC-USD,sell,105109.02,0.00000000";

        let order = CoinbaseOrder::from_csv_line(line).unwrap();
        assert!(order.is_cancellation());
    }

    #[test]
    fn test_parse_pyth_price() {
        let line = "1763040722687,2025-11-13T13:32:02.687390+00:00,e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43,BTC/USD,102578.20982595,36.12254814,-8,1763040721";

        let price = PythPrice::from_csv_line(line).unwrap();
        assert_eq!(price.timestamp_millis, 1763040722687);
        assert_eq!(price.price, 102578.20982595);
        assert_eq!(price.confidence, 36.12254814);
        assert_eq!(price.expo, -8);
    }
}
