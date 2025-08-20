pub const CREATE_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS trades (
    quote_denom String,
    base_denom String,
    addr String,
    direction UInt8,
    filled_base UInt128,
    filled_quote UInt128,
    refund_base UInt128,
    refund_quote UInt128,
    fee_base UInt128,
    fee_quote UInt128,
    clearing_price UInt128,
    created_at DateTime64(6),
    block_height UInt64,
    INDEX idx_addr addr TYPE bloom_filter GRANULARITY 1,
    INDEX idx_pair (base_denom, quote_denom) TYPE minmax GRANULARITY 1
) ENGINE = MergeTree()
ORDER BY (base_denom, quote_denom, block_height)
PARTITION BY toYYYYMM(created_at);
"#;

#[derive(Default)]
pub struct Migration;

impl Migration {
    pub fn migrations() -> Vec<String> {
        vec![CREATE_TABLE.to_string()]
    }
}
