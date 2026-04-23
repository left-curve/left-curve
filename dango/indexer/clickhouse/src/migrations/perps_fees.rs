pub const PERPS_FEES_TABLE: &str = r#"
  CREATE TABLE IF NOT EXISTS perps_fees (
    block_height UInt64,
    created_at DateTime64(6),
    protocol_fee UInt128,
    vault_fee UInt128,
    referee_rebate UInt128,
    referrer_payout UInt128,
    fee_events_count UInt32
  ) ENGINE = MergeTree()
  PARTITION BY toYYYYMM(created_at)
  ORDER BY (created_at, block_height)
"#;

/// Hourly pre-aggregation of `perps_fees`. Populated automatically by the
/// `perps_fees_hourly_mv` materialized view. Queries with a time range ≥ 3
/// days read from here instead of the per-block table for a ~4-orders-of-
/// magnitude row-count reduction.
pub const PERPS_FEES_HOURLY_TABLE: &str = r#"
  CREATE TABLE IF NOT EXISTS perps_fees_hourly (
    hour DateTime,
    protocol_fee UInt128,
    vault_fee UInt128,
    referee_rebate UInt128,
    referrer_payout UInt128,
    fee_events_count UInt64
  ) ENGINE = SummingMergeTree()
  PARTITION BY toYYYYMM(hour)
  ORDER BY hour
"#;

/// Trigger-style materialized view: each INSERT into `perps_fees` is
/// projected onto its `toStartOfHour(created_at)` bucket and appended to
/// `perps_fees_hourly`. `SummingMergeTree` collapses rows that share the
/// same `hour` on background merge, so queries aggregate with a plain
/// `sum()` regardless of merge state.
pub const PERPS_FEES_HOURLY_MV: &str = r#"
  CREATE MATERIALIZED VIEW IF NOT EXISTS perps_fees_hourly_mv
  TO perps_fees_hourly AS
  SELECT
    toStartOfHour(created_at) AS hour,
    protocol_fee,
    vault_fee,
    referee_rebate,
    referrer_payout,
    toUInt64(fee_events_count) AS fee_events_count
  FROM perps_fees
"#;

pub fn migrations() -> Vec<String> {
    vec![
        PERPS_FEES_TABLE.to_string(),
        PERPS_FEES_HOURLY_TABLE.to_string(),
        PERPS_FEES_HOURLY_MV.to_string(),
    ]
}
