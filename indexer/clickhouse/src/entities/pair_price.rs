#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, SimpleObject},
    grug_types::Timestamp,
};
use {
    chrono::{DateTime, Utc},
    clickhouse::Row,
    grug::{Udec128, Uint128},
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Row, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "PairPrice"))]
pub struct PairPrice {
    #[cfg_attr(feature = "async-graphql", graphql(name = "quoteDenom"))]
    pub quote_denom: String,
    #[cfg_attr(feature = "async-graphql", graphql(name = "baseDenom"))]
    pub base_denom: String,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    // #[serde(with = "udec128")]
    pub clearing_price: Udec128,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub volume_base: Uint128,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub volume_quote: Uint128,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub created_at: DateTime<Utc>,
    #[cfg_attr(feature = "async-graphql", graphql(name = "blockHeight"))]
    pub block_height: u64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl PairPrice {
    /// Returns the block timestamp in ISO 8601 format with time zone.
    async fn created_at(&self) -> String {
        Timestamp::from(self.created_at.naive_utc()).to_rfc3339_string()
    }

    // /// Returns the clearing price of the pair price.
    // async fn clearing_price(&self) -> BigDecimal {
    //     BigDecimal::from(self.clearing_price)
    // }

    // Returns the volume of the pair price.
    // async fn volume(&self) -> BigDecimal {
    //     BigDecimal::from(self.volume)
    // }
}

// pub mod udec128 {
//     use {
//         grug::Udec128,
//         serde::{
//             de::{Deserialize, Deserializer},
//             ser::{Serialize, Serializer},
//         },
//     };

//     /// evm U256 is represented in big-endian, but ClickHouse expects little-endian
//     pub fn serialize<S: Serializer>(u: &Udec128, serializer: S) -> Result<S::Ok, S::Error> {
//         // let buf: [u8; 32] = u.to_le_bytes();
//         // buf.serialize(serializer)
//         todo!()
//     }

//     /// ClickHouse stores U256 in little-endian
//     pub fn deserialize<'de, D>(deserializer: D) -> Result<Udec128, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         // let buf: [u8; 32] = Deserialize::deserialize(deserializer)?;
//         // Ok(Udec128::from_le_bytes(buf))
//         todo!()
//     }
// }
