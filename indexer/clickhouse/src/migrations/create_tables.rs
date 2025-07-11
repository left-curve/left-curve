pub const CREATE_TABLES: &str = r#"
            CREATE OR REPLACE TABLE pair_prices (
                quote_denom String,
                base_denom String,
                clearing_price UInt128,
                volume_base UInt128,
                volume_quote UInt128,
                created_at DateTime64(6),
                block_height UInt64
            ) ENGINE = MergeTree() -- ENGINE = MergeTree() is efficient for time-series data with the given ORDER BY.
            ORDER BY (quote_denom, base_denom, created_at)
            "#;

#[allow(dead_code)]
pub const CREATE_MATERIALIZED_VIEWS_1M: &str = r#"
            -- 1m target table: Pre-aggregated OHLCV data for 1-minute intervals.
            -- This is populated automatically by the materialized view.
            CREATE TABLE pair_prices_1m (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6), -- Start of the 1m interval
                open UInt128,
                high UInt128,
                low UInt128,
                close UInt128,
                volume_base UInt128,
                volume_quote UInt128
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, time_start)

            -- 1m materialized view: Automatically aggregates from pair_prices into pair_prices_1m.
            -- Uses toStartOfMinute (shorthand for INTERVAL 1 MINUTE).
            CREATE MATERIALIZED VIEW pair_prices_1m_mv TO pair_prices_1m AS
            SELECT
                quote_denom,
                base_denom,
                toStartOfMinute(created_at) AS time_start,
                argMin(clearing_price, created_at) AS open,
                max(clearing_price) AS high,
                min(clearing_price) AS low,
                argMax(clearing_price, created_at) AS close,
                sum(volume_base) AS volume_base,
                sum(volume_quote) AS volume_quote
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#;
