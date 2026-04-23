#[cfg(feature = "async-graphql")]
use {
    crate::{
        entities::{graphql_decimal::GraphqlBigDecimal, pair_price::dec},
        error::Result,
    },
    async_graphql::{ComplexObject, SimpleObject},
    bigdecimal::{BigDecimal, num_bigint::BigInt},
    grug::Inner,
    grug_types::Timestamp,
};
use {
    chrono::{DateTime, Utc},
    clickhouse::Row,
    grug::Udec128_6,
    serde::{Deserialize, Serialize},
};

/// One row per block that emitted at least one `FeeDistributed` event.
/// Values are pre-aggregated across all events in the block so that range
/// queries become a single streaming SUM.
///
/// Fee flow: the total fee paid by a user is `protocol_fee + vault_fee`.
/// The `vault_fee` is gross — a portion is redistributed as referral
/// commissions (`referee_rebate` back to the payer, `referrer_payout` up
/// the referrer chain). Net vault retention is
/// `vault_fee - referee_rebate - referrer_payout`.
#[derive(Debug, Row, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct PerpsFees {
    pub block_height: u64,
    #[serde(with = "clickhouse::serde::chrono::datetime64::micros")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::entities::pair_price::dec")]
    pub protocol_fee: Udec128_6,
    #[serde(with = "crate::entities::pair_price::dec")]
    pub vault_fee: Udec128_6,
    #[serde(with = "crate::entities::pair_price::dec")]
    pub referee_rebate: Udec128_6,
    #[serde(with = "crate::entities::pair_price::dec")]
    pub referrer_payout: Udec128_6,
    /// Number of `FeeDistributed` events aggregated into this row. `u32`
    /// suffices per-block; aggregate queries widen to `u64` to avoid
    /// overflow across long windows.
    pub fee_events_count: u32,
}

#[cfg(feature = "async-graphql")]
#[derive(Debug, Row, Deserialize)]
struct FeesAggregateRow {
    #[serde(with = "dec")]
    protocol_fee: Udec128_6,
    #[serde(with = "dec")]
    vault_fee: Udec128_6,
    #[serde(with = "dec")]
    referee_rebate: Udec128_6,
    #[serde(with = "dec")]
    referrer_payout: Udec128_6,
    fee_events_count: u64,
}

/// Aggregated fee/revenue totals over a `[from, to]` window.
#[cfg(feature = "async-graphql")]
#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct PerpsFeesAndRevenue {
    #[graphql(skip)]
    pub from: DateTime<Utc>,
    #[graphql(skip)]
    pub to: DateTime<Utc>,
    #[graphql(skip)]
    pub protocol_fee: Udec128_6,
    #[graphql(skip)]
    pub vault_fee: Udec128_6,
    #[graphql(skip)]
    pub referee_rebate: Udec128_6,
    #[graphql(skip)]
    pub referrer_payout: Udec128_6,
    #[graphql(name = "feeEventsCount")]
    pub fee_events_count: u64,
}

#[cfg(feature = "async-graphql")]
impl PerpsFeesAndRevenue {
    pub async fn fetch(
        clickhouse_client: &clickhouse::Client,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Self> {
        // Explicit `toUInt128` / `toUInt64` keeps the return type stable: ClickHouse
        // widens `sum(UInt128)` to `UInt256`, which the crate would fail to
        // deserialize into `u128`. Fees never approach 2^128 at USD·10^6 scale.
        let query = r#"
            SELECT
                toUInt128(sum(protocol_fee))    AS protocol_fee,
                toUInt128(sum(vault_fee))       AS vault_fee,
                toUInt128(sum(referee_rebate))  AS referee_rebate,
                toUInt128(sum(referrer_payout)) AS referrer_payout,
                toUInt64(sum(fee_events_count)) AS fee_events_count
            FROM perps_fees
            WHERE created_at >= toDateTime64(?, 6)
              AND created_at <= toDateTime64(?, 6)
        "#;

        let row: FeesAggregateRow = clickhouse_client
            .query(query)
            .bind(from.timestamp_micros())
            .bind(to.timestamp_micros())
            .fetch_one()
            .await?;

        Ok(Self {
            from,
            to,
            protocol_fee: row.protocol_fee,
            vault_fee: row.vault_fee,
            referee_rebate: row.referee_rebate,
            referrer_payout: row.referrer_payout,
            fee_events_count: row.fee_events_count,
        })
    }
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl PerpsFeesAndRevenue {
    /// ISO 8601 timestamp of the lower bound (inclusive).
    async fn from(&self) -> String {
        Timestamp::from(self.from.naive_utc()).to_rfc3339_string()
    }

    /// ISO 8601 timestamp of the upper bound (inclusive).
    async fn to(&self) -> String {
        Timestamp::from(self.to.naive_utc()).to_rfc3339_string()
    }

    async fn protocol_fee(&self) -> GraphqlBigDecimal {
        udec128_6_to_big_decimal(&self.protocol_fee)
    }

    async fn vault_fee(&self) -> GraphqlBigDecimal {
        udec128_6_to_big_decimal(&self.vault_fee)
    }

    async fn referee_rebate(&self) -> GraphqlBigDecimal {
        udec128_6_to_big_decimal(&self.referee_rebate)
    }

    async fn referrer_payout(&self) -> GraphqlBigDecimal {
        udec128_6_to_big_decimal(&self.referrer_payout)
    }
}

#[cfg(feature = "async-graphql")]
fn udec128_6_to_big_decimal(v: &Udec128_6) -> GraphqlBigDecimal {
    let inner_value = v.inner();
    let bigint = BigInt::from(*inner_value);
    BigDecimal::new(bigint, 6).normalized().into()
}
