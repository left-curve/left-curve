use {
    grug::Uint128,
    serde::{de, ser},
    std::ops::{Deref, DerefMut, DivAssign},
};

/// Volume is a wrapper around `grug::Uint128`, but serialize as a number.
/// Volume -> Uint128 -> u128
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub struct Volume(Uint128);

impl Deref for Volume {
    type Target = Uint128;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Volume {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DivAssign for Volume {
    fn div_assign(&mut self, rhs: Self) {
        self.0 /= rhs.0;
    }
}

impl From<Uint128> for Volume {
    fn from(value: Uint128) -> Self {
        Self(value)
    }
}

impl ser::Serialize for Volume {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        self.0.0.serialize(serializer)
    }
}

impl<'de> de::Deserialize<'de> for Volume {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let inner = <_ as de::Deserialize<'de>>::deserialize(deserializer)?;
        Ok(Self(grug::Uint128::new(inner)))
    }
}
