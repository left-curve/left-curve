use {
    bigdecimal::{
        BigDecimal,
        num_bigint::{BigInt, Sign},
    },
    bnum::types::U256,
    grug::{Bytable, Inner},
    serde::{de, ser},
    std::ops::{Deref, DerefMut},
};

// Using my own struct so I can use my own serde implementation since grug will serialize as a string
// and clickhouse expects a number.

/// Dec is a wrapper around `grug::Dec`, but serialize as a number.
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord, Hash)]
pub struct Dec<U>(U);

impl<U> Deref for Dec<U> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<U> DerefMut for Dec<U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<U> From<U> for Dec<U> {
    fn from(value: U) -> Self {
        Self(value)
    }
}

impl<U, const S: u32> ser::Serialize for Dec<grug::Dec<U, S>>
where
    U: ser::Serialize,
{
    fn serialize<S2>(&self, serializer: S2) -> Result<S2::Ok, S2::Error>
    where
        S2: ser::Serializer,
    {
        self.0.inner().serialize(serializer)
    }
}

impl<'de, U, const S: u32> de::Deserialize<'de> for Dec<grug::Dec<U, S>>
where
    U: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let inner: U = <_ as de::Deserialize<'de>>::deserialize(deserializer)?;
        let uint = grug::Int::new(inner);
        let dec = grug::Dec::raw(uint);
        Ok(Self(dec))
    }
}

// Helper function to convert u128 to BigInt
fn u128_to_bigint(value: u128) -> BigInt {
    BigInt::from(value)
}

// Helper function to convert U256 to BigInt
fn u256_to_bigint(value: U256) -> BigInt {
    let bytes = value.to_be_bytes();
    BigInt::from_bytes_be(Sign::Plus, &bytes)
}

// Implement conversion for specific types
impl From<Dec<grug::Dec<u128, 18>>> for BigDecimal {
    fn from(dec: Dec<grug::Dec<u128, 18>>) -> Self {
        let inner_value = *dec.0.inner();
        let bigint = u128_to_bigint(inner_value);
        BigDecimal::new(bigint, 18)
    }
}

// Implement conversion for specific types
impl From<Dec<grug::Dec<u128, 6>>> for BigDecimal {
    fn from(dec: Dec<grug::Dec<u128, 6>>) -> Self {
        let inner_value = *dec.0.inner();
        let bigint = u128_to_bigint(inner_value);
        BigDecimal::new(bigint, 6)
    }
}

impl From<Dec<grug::Dec<U256, 18>>> for BigDecimal {
    fn from(dec: Dec<grug::Dec<U256, 18>>) -> Self {
        let inner_value = *dec.0.inner();
        let bigint = u256_to_bigint(inner_value);
        BigDecimal::new(bigint, 18)
    }
}

impl From<Dec<grug::Dec<u128, 24>>> for BigDecimal {
    fn from(dec: Dec<grug::Dec<u128, 24>>) -> Self {
        let inner_value = *dec.0.inner();
        let bigint = u128_to_bigint(inner_value);
        BigDecimal::new(bigint, 24)
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        assertor::*,
        grug::{NumberConst, Udec128, Udec256},
    };

    #[test]
    fn test_volume() {
        let dec = Dec::<Udec128>::from(Udec128::ZERO);
        let serialized = serde_json::to_string(&dec).unwrap();
        let deserialized: Dec<Udec128> = serde_json::from_str(&serialized).unwrap();

        assert_that!(dec).is_equal_to(deserialized);

        let dec = Dec::<Udec256>::from(Udec256::ZERO);
        let serialized = serde_json::to_string(&dec).unwrap();
        let deserialized: Dec<Udec256> = serde_json::from_str(&serialized).unwrap();

        assert_that!(dec).is_equal_to(deserialized);
    }

    #[test]
    fn test_bigdecimal_conversion() {
        // Test Udec128 conversion
        let dec = Dec::<Udec128>::from(Udec128::new(123456789));
        let bigdecimal: BigDecimal = dec.into();
        assert_eq!(bigdecimal.to_string(), "123456789.000000000000000000");

        // Test Udec256 conversion
        let dec = Dec::<Udec256>::from(Udec256::new(987654321));
        let bigdecimal: BigDecimal = dec.into();
        assert_eq!(bigdecimal.to_string(), "987654321.000000000000000000");
    }
}
