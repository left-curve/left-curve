use {
    super::pair_price::dec,
    chrono::{DateTime, Utc},
    clickhouse::Row,
    dango_types::dex::Direction,
    grug::{Udec128_6, Udec128_24},
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, SimpleObject},
    bigdecimal::{BigDecimal, num_bigint::BigInt},
    grug::Inner,
    grug_types::Timestamp,
};

#[derive(Debug, Row, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "Trade"))]
pub struct Trade {
    pub addr: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "quoteDenom"))]
    pub quote_denom: String,

    #[cfg_attr(feature = "async-graphql", graphql(name = "baseDenom"))]
    pub base_denom: String,

    #[serde(with = "direction")]
    pub direction: Direction,

    #[serde(with = "dec")]
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub filled_base: Udec128_6,

    #[serde(with = "dec")]
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub filled_quote: Udec128_6,

    #[serde(with = "dec")]
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub refund_base: Udec128_6,

    #[serde(with = "dec")]
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub refund_quote: Udec128_6,

    #[serde(with = "dec")]
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub fee_base: Udec128_6,

    #[serde(with = "dec")]
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub fee_quote: Udec128_6,

    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "dec")]
    pub clearing_price: Udec128_24,

    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "clickhouse::serde::chrono::datetime64::micros")]
    pub created_at: DateTime<Utc>,

    #[cfg_attr(feature = "async-graphql", graphql(name = "blockHeight"))]
    pub block_height: u64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Trade {
    /// Returns the block timestamp in ISO 8601 format with time zone.
    async fn created_at(&self) -> String {
        Timestamp::from(self.created_at.naive_utc()).to_rfc3339_string()
    }

    async fn filled_base(&self) -> BigDecimal {
        let inner_value = self.filled_base.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized()
    }

    async fn filled_quote(&self) -> BigDecimal {
        let inner_value = self.filled_quote.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized()
    }

    async fn refund_base(&self) -> BigDecimal {
        let inner_value = self.refund_base.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized()
    }

    async fn refund_quote(&self) -> BigDecimal {
        let inner_value = self.refund_quote.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized()
    }

    async fn fee_base(&self) -> BigDecimal {
        let inner_value = self.fee_base.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized()
    }

    async fn fee_quote(&self) -> BigDecimal {
        let inner_value = self.fee_quote.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized()
    }

    async fn clearing_price(&self) -> BigDecimal {
        let inner_value = self.clearing_price.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 24).normalized()
    }
}

/// This will serialize and deserialize direction as u8, which is needed
/// for clickhouse.
pub mod direction {
    use {
        super::Direction,
        serde::{
            Deserialize,
            de::{self, Deserializer},
            ser::{Serialize, Serializer},
        },
    };

    pub fn serialize<S>(direction: &Direction, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val: u8 = match direction {
            Direction::Bid => 0,
            Direction::Ask => 1,
        };
        val.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Direction, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = u8::deserialize(deserializer)?;
        match val {
            0 => Ok(Direction::Bid),
            1 => Ok(Direction::Ask),
            _ => Err(de::Error::custom(format!("Invalid direction: {}", val))),
        }
    }
}
