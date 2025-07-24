use {
    crate::error::Result,
    chrono::{DateTime, Utc},
    clickhouse::Row,
    grug::{Udec128_6, Udec128_24},
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, SimpleObject},
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
    #[serde(with = "dec")]
    pub clearing_price: Udec128_24,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "dec")]
    pub volume_base: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "dec")]
    pub volume_quote: Udec128_6,
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
}

impl PairPrice {
    pub async fn last_prices(clickhouse_client: &clickhouse::Client) -> Result<Vec<PairPrice>> {
        let query = r#"
          SELECT
            quote_denom,
            base_denom,
            clearing_price,
            volume_base,
            volume_quote,
            created_at,
            block_height
          FROM (
              SELECT *,
                  row_number() OVER (
                      PARTITION BY quote_denom, base_denom
                      ORDER BY block_height DESC
                  ) AS rn
              FROM pair_prices
          )
          WHERE rn = 1
        "#;

        Ok(clickhouse_client.query(query).fetch_all().await?)
    }

    pub async fn cleanup_old_synthetic_data(
        clickhouse_client: &clickhouse::Client,
        current_block: u64,
    ) -> Result<()> {
        let query = "DELETE FROM pair_prices WHERE volume_base = 0 AND volume_quote = 0 AND block_height = ?";
        clickhouse_client
            .query(query)
            .bind(current_block - 1)
            .execute()
            .await?;
        Ok(())
    }
}

/// This will serialize and deserialize the decimals as u128, which is needed
/// for clickhouse.
pub mod dec {
    use {
        grug::Inner,
        serde::{
            de::{self, Deserializer},
            ser::{Serialize, Serializer},
        },
    };

    pub fn serialize<S, U, const D: u32>(
        dec: &grug::Dec<U, D>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        U: Serialize,
    {
        dec.inner().serialize(serializer)
    }

    pub fn deserialize<'de, D, U, const S: u32>(
        deserializer: D,
    ) -> Result<grug::Dec<U, S>, D::Error>
    where
        D: Deserializer<'de>,
        U: de::Deserialize<'de>,
    {
        let inner: U = <_ as de::Deserialize<'de>>::deserialize(deserializer)?;
        let uint = grug::Int::new(inner);
        let dec = grug::Dec::raw(uint);
        Ok(dec)
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        assertor::*,
        chrono::SubsecRound,
        grug::{Dec128_6, NumberConst, Udec128, Udec256, Uint128, Uint256},
    };

    #[test]
    fn serde_pair_price() {
        let pair_price = PairPrice {
            quote_denom: "USDC".to_string(),
            base_denom: "USDT".to_string(),
            clearing_price: Udec128_24::MAX,
            volume_base: Udec128_6::MAX,
            volume_quote: Udec128_6::MAX,
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
    #[test]
    fn test_bnum_u256() {
        let udec256 = serde_json::json!({"max": bnum::types::U256::MAX, "min": bnum::types::U256::MIN, "one": bnum::types::U256::ONE});
        let serialized = serde_json::to_string(&udec256).unwrap();
        let _deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        // println!("serialized = {serialized}",);
        // println!("deserialized = {_deserialized:#?}",);
    }

    /// This allows seeing the serialized value of all types.
    /// This test matters, because it's the only way to test that the serde_json
    /// implementation is correct. We'll use serde to inject data into clickhouse.
    /// This requires the `arbitrary_precision` feature to be working.
    #[test]
    fn test_all_types() {
        let all_types = serde_json::json!({
            "udec128": Udec128::MAX,
            "udec256": Udec256::MAX,
            "uint128": Uint128::MAX,
            "uint256": Uint256::MAX,
            "volume": Dec128_6::MAX,
            "clearing_price": Udec128::MAX,
            "bnum_u128": bnum::types::U128::ONE,
            "bnum_u256": bnum::types::U256::ONE,
            "u128": u128::MAX,
        });
        let serialized = serde_json::to_string(&all_types).unwrap();
        let _deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        // println!("serialized = {serialized}",);
        // println!("deserialized = {deserialized:#?}",);

        // let clearing_price: ClearingPrice =
        //     serde_json::from_value(deserialized["clearing_price"].clone()).unwrap();

        // println!("clearing_price = {clearing_price:#?}",);
    }
}
