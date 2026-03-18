pub const PERPS_PAIR_PRICE_TABLE: &str = r#"
  CREATE TABLE IF NOT EXISTS perps_pair_prices (
    pair_id String,
    high UInt128,
    low UInt128,
    close UInt128,
    volume UInt128,
    volume_usd UInt128,
    created_at DateTime64(6),
    block_height UInt64
  ) ENGINE = MergeTree()
  PARTITION BY toYYYYMM(created_at)
  ORDER BY (pair_id, block_height)
"#;

pub const PERPS_CANDLE_TABLE: &str = r#"
  CREATE TABLE IF NOT EXISTS perps_candles (
    pair_id String,
    time_start DateTime64(6),
    open UInt128,
    high UInt128,
    low UInt128,
    close UInt128,
    volume UInt128,
    volume_usd UInt128,
    min_block_height UInt64,
    max_block_height UInt64,
    interval String
  ) ENGINE = ReplacingMergeTree(max_block_height)
  PARTITION BY (interval, toYYYYMM(time_start))
  ORDER BY (pair_id, interval, time_start)
"#;

pub fn migrations() -> Vec<String> {
    vec![
        PERPS_CANDLE_TABLE.to_string(),
        PERPS_PAIR_PRICE_TABLE.to_string(),
    ]
}
