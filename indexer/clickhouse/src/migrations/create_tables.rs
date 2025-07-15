pub const MIGRATIONS: [&str; 25] = [
    CREATE_TABLES,
    // 1 second
    CREATE_PAIR_PRICES_1S_TABLE,
    DROP_PAIR_PRICES_1S_MV,
    CREATE_PAIR_PRICES_1S_MV,
    // 1 minute
    CREATE_PAIR_PRICES_1M_TABLE,
    DROP_PAIR_PRICES_1M_MV,
    CREATE_PAIR_PRICES_1M_MV,
    // 5 minutes
    CREATE_PAIR_PRICES_5M_TABLE,
    DROP_PAIR_PRICES_5M_MV,
    CREATE_PAIR_PRICES_5M_MV,
    // 15 minutes
    CREATE_PAIR_PRICES_15M_TABLE,
    DROP_PAIR_PRICES_15M_MV,
    CREATE_PAIR_PRICES_15M_MV,
    // 1 hour
    CREATE_PAIR_PRICES_1H_TABLE,
    DROP_PAIR_PRICES_1H_MV,
    CREATE_PAIR_PRICES_1H_MV,
    // 4 hours
    CREATE_PAIR_PRICES_4H_TABLE,
    DROP_PAIR_PRICES_4H_MV,
    CREATE_PAIR_PRICES_4H_MV,
    // 1 day
    CREATE_PAIR_PRICES_1D_TABLE,
    DROP_PAIR_PRICES_1D_MV,
    CREATE_PAIR_PRICES_1D_MV,
    // 1 week
    CREATE_PAIR_PRICES_1W_TABLE,
    DROP_PAIR_PRICES_1W_MV,
    CREATE_PAIR_PRICES_1W_MV,
];

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

pub const CREATE_PAIR_PRICES_1S_TABLE: &str = r#"
            -- 1s target table: Pre-aggregated OHLCV data for 1-second intervals.
            -- This is populated automatically by the materialized view.
            CREATE OR REPLACE TABLE pair_prices_1s (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6), -- Start of the 1s interval
                open UInt128,
                high UInt128,
                low UInt128,
                close UInt128,
                volume_base UInt128,
                volume_quote UInt128
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, time_start)
"#;

pub const DROP_PAIR_PRICES_1S_MV: &str = r#"
            DROP VIEW IF EXISTS pair_prices_1s_mv
"#;

pub const CREATE_PAIR_PRICES_1S_MV: &str = r#"
            -- 1s materialized view: Automatically aggregates from pair_prices into pair_prices_1s.
            CREATE MATERIALIZED VIEW pair_prices_1s_mv TO pair_prices_1s AS
            SELECT
                quote_denom,
                base_denom,
                toStartOfSecond(created_at) AS time_start,
                argMin(clearing_price, created_at) AS open,
                max(clearing_price) AS high,
                min(clearing_price) AS low,
                argMax(clearing_price, created_at) AS close,
                sum(volume_base) AS volume_base,
                sum(volume_quote) AS volume_quote
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#;

pub const CREATE_PAIR_PRICES_1M_TABLE: &str = r#"
            -- 1m target table: Pre-aggregated OHLCV data for 1-minute intervals.
            -- This is populated automatically by the materialized view.
            CREATE OR REPLACE TABLE pair_prices_1m (
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
"#;

pub const DROP_PAIR_PRICES_1M_MV: &str = r#"
            DROP VIEW IF EXISTS pair_prices_1m_mv
"#;

pub const CREATE_PAIR_PRICES_1M_MV: &str = r#"
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

// 5 minutes
pub const CREATE_PAIR_PRICES_5M_TABLE: &str = r#"
            CREATE OR REPLACE TABLE pair_prices_5m (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6),
                open UInt128,
                high UInt128,
                low UInt128,
                close UInt128,
                volume_base UInt128,
                volume_quote UInt128
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, time_start)
"#;

pub const DROP_PAIR_PRICES_5M_MV: &str = r#"
            DROP VIEW IF EXISTS pair_prices_5m_mv
"#;

pub const CREATE_PAIR_PRICES_5M_MV: &str = r#"
            CREATE MATERIALIZED VIEW pair_prices_5m_mv TO pair_prices_5m AS
            SELECT
                quote_denom,
                base_denom,
                toStartOfFiveMinute(created_at) AS time_start,
                argMin(clearing_price, created_at) AS open,
                max(clearing_price) AS high,
                min(clearing_price) AS low,
                argMax(clearing_price, created_at) AS close,
                sum(volume_base) AS volume_base,
                sum(volume_quote) AS volume_quote
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#;

// 15 minutes
pub const CREATE_PAIR_PRICES_15M_TABLE: &str = r#"
            CREATE OR REPLACE TABLE pair_prices_15m (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6),
                open UInt128,
                high UInt128,
                low UInt128,
                close UInt128,
                volume_base UInt128,
                volume_quote UInt128
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, time_start)
"#;

pub const DROP_PAIR_PRICES_15M_MV: &str = r#"
            DROP VIEW IF EXISTS pair_prices_15m_mv
"#;

pub const CREATE_PAIR_PRICES_15M_MV: &str = r#"
            CREATE MATERIALIZED VIEW pair_prices_15m_mv TO pair_prices_15m AS
            SELECT
                quote_denom,
                base_denom,
                toStartOfFifteenMinutes(created_at) AS time_start,
                argMin(clearing_price, created_at) AS open,
                max(clearing_price) AS high,
                min(clearing_price) AS low,
                argMax(clearing_price, created_at) AS close,
                sum(volume_base) AS volume_base,
                sum(volume_quote) AS volume_quote
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#;

// 1 hour
pub const CREATE_PAIR_PRICES_1H_TABLE: &str = r#"
            CREATE OR REPLACE TABLE pair_prices_1h (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6),
                open UInt128,
                high UInt128,
                low UInt128,
                close UInt128,
                volume_base UInt128,
                volume_quote UInt128
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, time_start)
"#;

pub const DROP_PAIR_PRICES_1H_MV: &str = r#"
            DROP VIEW IF EXISTS pair_prices_1h_mv
"#;

pub const CREATE_PAIR_PRICES_1H_MV: &str = r#"
            CREATE MATERIALIZED VIEW pair_prices_1h_mv TO pair_prices_1h AS
            SELECT
                quote_denom,
                base_denom,
                toStartOfHour(created_at) AS time_start,
                argMin(clearing_price, created_at) AS open,
                max(clearing_price) AS high,
                min(clearing_price) AS low,
                argMax(clearing_price, created_at) AS close,
                sum(volume_base) AS volume_base,
                sum(volume_quote) AS volume_quote
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#;

// 4 hours
pub const CREATE_PAIR_PRICES_4H_TABLE: &str = r#"
            CREATE OR REPLACE TABLE pair_prices_4h (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6),
                open UInt128,
                high UInt128,
                low UInt128,
                close UInt128,
                volume_base UInt128,
                volume_quote UInt128
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, time_start)
"#;

pub const DROP_PAIR_PRICES_4H_MV: &str = r#"
            DROP VIEW IF EXISTS pair_prices_4h_mv
"#;

pub const CREATE_PAIR_PRICES_4H_MV: &str = r#"
            CREATE MATERIALIZED VIEW pair_prices_4h_mv TO pair_prices_4h AS
            SELECT
                quote_denom,
                base_denom,
                toStartOfInterval(created_at, INTERVAL 4 HOUR) AS time_start,
                argMin(clearing_price, created_at) AS open,
                max(clearing_price) AS high,
                min(clearing_price) AS low,
                argMax(clearing_price, created_at) AS close,
                sum(volume_base) AS volume_base,
                sum(volume_quote) AS volume_quote
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#;

// 1 day
pub const CREATE_PAIR_PRICES_1D_TABLE: &str = r#"
            CREATE OR REPLACE TABLE pair_prices_1d (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6),
                open UInt128,
                high UInt128,
                low UInt128,
                close UInt128,
                volume_base UInt128,
                volume_quote UInt128
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, time_start)
"#;

pub const DROP_PAIR_PRICES_1D_MV: &str = r#"
            DROP VIEW IF EXISTS pair_prices_1d_mv
"#;

pub const CREATE_PAIR_PRICES_1D_MV: &str = r#"
            CREATE MATERIALIZED VIEW pair_prices_1d_mv TO pair_prices_1d AS
            SELECT
                quote_denom,
                base_denom,
                toStartOfDay(created_at) AS time_start,
                argMin(clearing_price, created_at) AS open,
                max(clearing_price) AS high,
                min(clearing_price) AS low,
                argMax(clearing_price, created_at) AS close,
                sum(volume_base) AS volume_base,
                sum(volume_quote) AS volume_quote
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#;

// 1 week
pub const CREATE_PAIR_PRICES_1W_TABLE: &str = r#"
            CREATE OR REPLACE TABLE pair_prices_1w (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6),
                open UInt128,
                high UInt128,
                low UInt128,
                close UInt128,
                volume_base UInt128,
                volume_quote UInt128
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, time_start)
"#;

pub const DROP_PAIR_PRICES_1W_MV: &str = r#"
            DROP VIEW IF EXISTS pair_prices_1w_mv
"#;

pub const CREATE_PAIR_PRICES_1W_MV: &str = r#"
            CREATE MATERIALIZED VIEW pair_prices_1w_mv TO pair_prices_1w AS
            SELECT
                quote_denom,
                base_denom,
                toStartOfWeek(created_at) AS time_start,
                argMin(clearing_price, created_at) AS open,
                max(clearing_price) AS high,
                min(clearing_price) AS low,
                argMax(clearing_price, created_at) AS close,
                sum(volume_base) AS volume_base,
                sum(volume_quote) AS volume_quote
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#;
