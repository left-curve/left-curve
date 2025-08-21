pub const PAIR_PRICE_TABLE: &str = r#"
  CREATE TABLE IF NOT EXISTS pair_prices (
    quote_denom String,
    base_denom String,
    clearing_price UInt128,
    volume_base UInt128,
    volume_quote UInt128,
    created_at DateTime64(6),
    block_height UInt64
  ) ENGINE = MergeTree()
  PARTITION BY toYYYYMM(created_at)
  ORDER BY (quote_denom, base_denom, block_height)
"#;

pub const CANDLE_TABLE: &str = r#"
  CREATE TABLE IF NOT EXISTS candles (
    quote_denom String,
    base_denom String,
    time_start DateTime64(6),
    open UInt128,
    high UInt128,
    low UInt128,
    close UInt128,
    volume_base UInt128,
    volume_quote UInt128,
    block_height UInt64,
    interval String
  ) ENGINE = ReplacingMergeTree(block_height)
  PARTITION BY (interval, toYYYYMM(time_start))
  ORDER BY (quote_denom, base_denom, interval, time_start)
"#;

pub fn migrations() -> Vec<String> {
    vec![CANDLE_TABLE.to_string(), PAIR_PRICE_TABLE.to_string()]
}
