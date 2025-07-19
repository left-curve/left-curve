use {crate::entities::candle::CandleInterval, strum::IntoEnumIterator};

pub const CREATE_TABLES: &str = r#"
            CREATE OR REPLACE TABLE pair_prices (
                quote_denom String,
                base_denom String,
                clearing_price UInt128,
                volume_base UInt128,
                volume_quote UInt128,
                created_at DateTime64(6),
                block_height UInt64
            ) ENGINE = MergeTree()
            ORDER BY (quote_denom, base_denom, block_height)
            "#;

#[derive(Default)]
pub struct MigrationBuilder {
    timeframes: Vec<(String, String)>,
}

impl MigrationBuilder {
    /// Add a simple timeframe with a standard ClickHouse function
    pub fn add_timeframe(mut self, timeframe: &str, clickhouse_fn: &str) -> Self {
        self.timeframes
            .push((timeframe.to_string(), clickhouse_fn.to_string()));
        self
    }

    /// Add a custom interval timeframe
    pub fn add_interval(mut self, timeframe: &str, interval: u32, unit: &str) -> Self {
        let clickhouse_fn = format!("toStartOfInterval(created_at, INTERVAL {interval} {unit})",);
        self.timeframes.push((timeframe.to_string(), clickhouse_fn));
        self
    }

    /// Build all migrations
    pub fn build(self) -> Vec<String> {
        let mut migrations = vec![CREATE_TABLES.to_string()];

        for (timeframe, time_fn) in &self.timeframes {
            migrations.push(self.create_table(timeframe));
            migrations.push(self.drop_view(timeframe));
            migrations.push(self.create_view(timeframe, time_fn));
        }

        migrations
    }

    fn create_table(&self, timeframe: &str) -> String {
        create_aggregated_table(timeframe)
    }

    fn drop_view(&self, timeframe: &str) -> String {
        drop_materialized_view(timeframe)
    }

    fn create_view(&self, timeframe: &str, time_function: &str) -> String {
        format!(
            r#"
            CREATE MATERIALIZED VIEW pair_prices_{timeframe}_mv TO pair_prices_{timeframe} AS
            SELECT
                quote_denom,
                base_denom,
                {time_function} AS time_start,
                argMinState(clearing_price, created_at) AS open,
                maxState(clearing_price) AS high,
                minState(clearing_price) AS low,
                argMaxState(clearing_price, created_at) AS close,
                sumState(volume_base) AS volume_base,
                sumState(volume_quote) AS volume_quote,
                maxState(block_height) AS block_height
            FROM pair_prices
            GROUP BY quote_denom, base_denom, time_start
"#,
        )
    }
}

fn create_aggregated_table(timeframe: &str) -> String {
    format!(
        r#"
            CREATE OR REPLACE TABLE pair_prices_{timeframe} (
                quote_denom String,
                base_denom String,
                time_start DateTime64(6),
                open AggregateFunction(argMin, UInt128, DateTime64(6)),
                high AggregateFunction(max, UInt128),
                low AggregateFunction(min, UInt128),
                close AggregateFunction(argMax, UInt128, DateTime64(6)),
                volume_base AggregateFunction(sum, UInt128),
                volume_quote AggregateFunction(sum, UInt128),
                block_height AggregateFunction(max, UInt64)
            ) ENGINE = AggregatingMergeTree()
            ORDER BY (quote_denom, base_denom, time_start)
"#
    )
}

fn drop_materialized_view(timeframe: &str) -> String {
    format!("DROP VIEW IF EXISTS pair_prices_{timeframe}_mv")
}

pub fn migrations() -> Vec<String> {
    let mut migration = MigrationBuilder::default();

    // NOTE: looping through all intervals on purpose, so when a new interval is
    // added, the migration fails if the method is not implemented
    for interval in CandleInterval::iter() {
        let method = match interval {
            CandleInterval::OneSecond => "toStartOfSecond(created_at)",
            CandleInterval::OneMinute => "toStartOfMinute(created_at)",
            CandleInterval::FiveMinutes => "toStartOfFiveMinute(created_at)",
            CandleInterval::FifteenMinutes => "toStartOfFifteenMinutes(created_at)",
            CandleInterval::OneHour => "toStartOfHour(created_at)",
            CandleInterval::FourHours => "toStartOfInterval(created_at, INTERVAL 4 HOUR)",
            CandleInterval::OneDay => "toStartOfDay(created_at)",
            CandleInterval::OneWeek => "toStartOfWeek(created_at)",
        };

        migration = migration.add_timeframe(interval.to_string().as_str(), method);
    }

    migration.build()
}
