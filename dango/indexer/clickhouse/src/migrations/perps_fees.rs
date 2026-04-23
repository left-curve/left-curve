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

pub fn migrations() -> Vec<String> {
    vec![PERPS_FEES_TABLE.to_string()]
}
