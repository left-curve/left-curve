# Market Data Replay for DEX Testing

This directory contains market data from Bitstamp for testing the DEX with realistic order flows.
Bitstamp was the only exchange that provided an open API for collecting real time order updates.

## Overview

The market data replay system allows you to:

1. Load historical order book updates from CSV files
2. Batch orders by time intervals to emulate blocks
3. Convert thh orders to Dango DEX-compatible format
4. Replay orders on the DEX

## Data File

- **coinbase_incoming_orders_BTCUSD_20251112_144231.csv**: Contains ~1.5M order book updates for BTC/USD from Coinbase
- Format: `timestamp_millis,timestamp_iso,event_type,product_id,side,price,size`

## Components

### Bitstamp Data Adapter (`bitstamp_data_adapter.rs`)

The adapter provides:

- **CSV Parsing**: Reads and parses Coinbase order data
- **Time-based Batching**: Groups orders by time intervals
- **Order Conversion**: Converts Coinbase orders to DEX `CreateOrderRequest` format
- **Transaction Creation**: Automatically creates and signs transactions ready for submission
