use {
    crate::{dec::Dec, entities::volume::Volume},
    chrono::{DateTime, Utc},
    clickhouse::Row,
    grug::{Udec128, Uint128},
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, SimpleObject},
    bigdecimal::BigDecimal,
    grug_types::Timestamp,
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
    pub clearing_price: Dec<Udec128>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub volume_base: Volume<Uint128>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub volume_quote: Volume<Uint128>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "clickhouse::serde::chrono::datetime64::micros")]
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

    // Returns the clearing price of the pair price.
    async fn clearing_price(&self) -> BigDecimal {
        BigDecimal::from(self.clearing_price.clone())
    }

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

#[cfg(test)]
mod test {
    use {
        super::*,
        assertor::*,
        chrono::SubsecRound,
        grug::{NumberConst, Udec128, Udec256, Uint128, Uint256},
    };

    #[test]
    fn serde_pair_price() {
        let pair_price = PairPrice {
            quote_denom: "USDC".to_string(),
            base_denom: "USDT".to_string(),
            clearing_price: Udec128::MAX.into(),
            volume_base: Uint128::MAX.into(),
            volume_quote: Uint128::MAX.into(),
            // On the CI I saw nanoseconds (9), on my computer it's milliseconds (6).
            created_at: Utc::now().trunc_subsecs(6),
            block_height: 1000000,
        };

        let serialized = serde_json::to_string(&pair_price).unwrap();
        let mut deserialized: PairPrice = serde_json::from_str(&serialized).unwrap();

        // On the CI I saw nanoseconds (9), on my computer it's milliseconds (6)..
        deserialized.created_at = deserialized.created_at.trunc_subsecs(6);

        // println!("serialized = {serialized}",);
        // println!("deserialized = {deserialized:#?}",);

        assert_that!(pair_price).is_equal_to(deserialized);
    }

    /// For when I'll need to switch to bnum for U256.
    #[ignore]
    #[test]
    fn test_bnum_u256() {
        let udec256 = serde_json::json!({"max": bnum::types::U256::MAX, "min": bnum::types::U256::MIN, "one": bnum::types::U256::ONE});
        let serialized = serde_json::to_string(&udec256).unwrap();
        let _deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        // println!("serialized = {serialized}",);
        // println!("deserialized = {_deserialized:#?}",);
    }

    /// This test matters, because it's the only way to test that the serde_json
    /// implementation is correct. We'll use serde to inject data into clickhouse.
    /// This requires the `arbitrary_precision` feature to be working.
    #[test]
    fn test_u128() {
        let u128 = serde_json::json!({"u128": u128::MAX});
        let serialized = serde_json::to_string(&u128).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let deserialized_u128: u128 = serde_json::from_value(deserialized["u128"].clone()).unwrap();

        // println!("serialized = {serialized:?}",);
        // println!("deserialized = {deserialized:?}",);
        // println!("u128 = {u128:?}",);
        // println!("deserialized_u128 = {deserialized_u128:?}",);

        assert_that!(deserialized_u128).is_equal_to(u128::MAX);
    }

    /// This allows seeing the serialized value of all types.
    #[test]
    fn test_all_types() {
        let all_types = serde_json::json!({
            "udec128": Udec128::MAX,
            "udec256": Udec256::MAX,
            "uint128": Uint128::MAX,
            "uint256": Uint256::MAX,
            "volume": Volume::from(Uint128::MAX),
            "clearing_price": Dec::<Udec128>::from(Udec128::MAX),
            "bnum_u128": bnum::types::U128::ONE,
            "bnum_u256": bnum::types::U256::ONE,
            "u128": u128::MAX,
        });
        let serialized = serde_json::to_string(&all_types).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        println!("serialized = {serialized}",);
        println!("deserialized = {deserialized:#?}",);

        // let clearing_price: ClearingPrice =
        //     serde_json::from_value(deserialized["clearing_price"].clone()).unwrap();

        // println!("clearing_price = {clearing_price:#?}",);
    }

    #[test]
    fn serde_volume() {
        let volume = serde_json::json!({"max": Volume::from(Uint128::MAX)});
        let serialized = serde_json::to_string(&volume).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let deserialized_volume: Volume<Uint128> =
            serde_json::from_value(deserialized["max"].clone()).unwrap();

        // println!("serialized = {serialized}",);
        // println!("deserialized = {deserialized:?}",);
        // println!("deserialized_volume = {deserialized_volume:?}",);

        assert_that!(deserialized["max"].is_number()).is_true();
        assert_that!(deserialized_volume).is_equal_to(Volume::from(Uint128::MAX));
    }

    #[test]
    fn serde_clearing_price() {
        let clearing_price = serde_json::json!({"max": Dec::from(Udec128::MAX)});
        let serialized = serde_json::to_string(&clearing_price).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let deserialized_clearing_price: Dec<Udec128> =
            serde_json::from_value(deserialized["max"].clone()).unwrap();

        // println!("serialized = {serialized}",);
        // println!("deserialized = {deserialized:#?}",);
        // println!("deserialized_clearing_price = {deserialized_clearing_price:#?}",);

        // Check that the serialized value is a number, this is needed for clickhouse.
        assert_that!(deserialized["max"].is_number()).is_true();
        assert_that!(deserialized_clearing_price).is_equal_to(Dec::<Udec128>::from(Udec128::MAX));
    }
}
