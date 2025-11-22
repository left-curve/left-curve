# Market Data Replay for DEX Testing

This directory contains market data from Bitstamp for testing the DEX with realistic order flows.
Bitstamp was the only exchange that provided an open API for collecting real time order updates.

## Overview

The market data replay system allows you to:

1. Load historical order book updates from CSV files
2. Batch orders by time intervals to emulate blocks
3. Convert the orders to Dango DEX-compatible format
4. Replay orders on the DEX

## Data Files

The data adapter requires two CSV files. One for the order book updates and one for the pyth oracle price during the same time period.

### Current data files

- `market_data/bitstamp/bitstamp_btcusd_20251118_132040.csv`: Contains 12021 order book updates for BTC/USD from Bitstamp collected during 50 seconds starting at 13:20:40 UTC on 2025-11-18.
- `market_data/bitstamp/pyth_btcusd_20251118_132040.csv`: Contains price updates for BTC/USD from Pyth collected during 50 seconds starting at 13:20:40 UTC on 2025-11-18.

## Components

### Bitstamp Data Adapter (`data_adapters/bitstamp_data_adapter.rs`)

The adapter provides:

- **CSV Parsing**: Reads and parses Coinbase order data
- **Time-based Batching**: Groups orders by time intervals
- **Order Conversion**: Converts Coinbase orders to DEX `CreateOrderRequest` format
- **Transaction Creation**: Automatically creates and signs transactions ready for submission

### Bitstamp Backtesting Test (`bitstamp.rs`)

The test needs to replay the orders and keep track of a few things when doing so.

1. The order ids for the Bitstamp orders must be mapped to the order ids used by the dango dex.
2. The order update data was collected from Bitstamp in real time and so it contains updates and cancelations of orders placed prior to the start time of data collection. Trying to cancel these orders on the dango dex will fail because the orders are not present in the order book.
3. The Dango DEX does not support changing orders, so we need to handle this case by cancelling the old order and creating a new one.
4. Dango DEX uses a frequent batch auction for filling orders and has passive liquidity (the testing of which is the whole point of this test). As such the order filling will behave differently to Bitstamp and some cancellations in the Bitstamp data might be for orders that have already been matched and cleared in the Dango DEX.

We do this by setting the block time to 0 seconds and the CronFrequency of the DEX contract to 1 ms. This allows us to execute multiple transactions without triggering the DEX cron job which performs the clearing of the order book. That way we can place all the orders for a specific time slot onto the book and after each collect the Dango DEX order id from the emitted events and connect it to the Bitstamp order ID. We can then trigger the cron job by fast forwarding the block time by the desired amount, which resolves the matching for the order book.
