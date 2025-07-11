use {
    grug::Udec128,
    serde::{de, ser},
    std::ops::{Deref, DerefMut},
};

// Using my own struct so I can use my own serde implementation since grug will serialize as a string
// and clickhouse expects a number.

/// ClearingPrice is a wrapper around `grug::Udec128`, but serialize as a number.
/// ClearingPrice -> Udec128 -> Uint128 -> u128
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub struct ClearingPrice(Udec128);

impl Deref for ClearingPrice {
    type Target = Udec128;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ClearingPrice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Udec128> for ClearingPrice {
    fn from(value: Udec128) -> Self {
        Self(value)
    }
}

impl ser::Serialize for ClearingPrice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        self.0.0.0.serialize(serializer)
    }
}

impl<'de> de::Deserialize<'de> for ClearingPrice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let inner: u128 = <_ as de::Deserialize<'de>>::deserialize(deserializer)?;
        Ok(Self(grug::Udec128::raw(grug::Uint128::new(inner))))
    }
}
