use {
    grug::Inner,
    serde::{de, ser},
    std::ops::{Deref, DerefMut},
};

// Using my own struct so I can use my own serde implementation since grug will serialize as a string
// and clickhouse expects a number.

/// ClearingPrice is a wrapper around `grug::Udec128`, but serialize as a number.
/// ClearingPrice -> Udec128 -> Uint128 -> u128
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub struct ClearingPrice<U>(U);

impl<U> Deref for ClearingPrice<U> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<U> DerefMut for ClearingPrice<U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<U> From<U> for ClearingPrice<U> {
    fn from(value: U) -> Self {
        Self(value)
    }
}

impl<U, const S: u32> ser::Serialize for ClearingPrice<grug::Dec<U, S>>
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

impl<'de, U, const S: u32> de::Deserialize<'de> for ClearingPrice<grug::Dec<U, S>>
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

#[cfg(test)]
mod test {
    use {
        super::*,
        assertor::*,
        grug::{NumberConst, Udec128, Udec256},
    };

    #[test]
    fn test_volume() {
        let clearing_price = ClearingPrice::<Udec128>::from(Udec128::ZERO);
        let serialized = serde_json::to_string(&clearing_price).unwrap();
        let deserialized: ClearingPrice<Udec128> = serde_json::from_str(&serialized).unwrap();

        assert_that!(clearing_price).is_equal_to(deserialized);

        let clearing_price = ClearingPrice::<Udec256>::from(Udec256::ZERO);
        let serialized = serde_json::to_string(&clearing_price).unwrap();
        let deserialized: ClearingPrice<Udec256> = serde_json::from_str(&serialized).unwrap();

        assert_that!(clearing_price).is_equal_to(deserialized);
    }
}
