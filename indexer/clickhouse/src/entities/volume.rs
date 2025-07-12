use {
    grug::Inner,
    serde::{de, ser},
    std::ops::{Deref, DerefMut, DivAssign},
};

/// Volume is a wrapper around `grug::Int<U>`, but serialize as a number.
/// Volume -> Int<U> -> U
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub struct Volume<U>(U);

impl<U> Deref for Volume<U> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<U> DerefMut for Volume<U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<U> DivAssign for Volume<U>
where
    U: DivAssign,
{
    fn div_assign(&mut self, rhs: Self) {
        self.0 /= rhs.0;
    }
}

impl<U> From<U> for Volume<U> {
    fn from(value: U) -> Self {
        Self(value)
    }
}

impl<U> ser::Serialize for Volume<grug::Int<U>>
where
    U: ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        self.0.inner().serialize(serializer)
    }
}

impl<'de, U> de::Deserialize<'de> for Volume<grug::Int<U>>
where
    U: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let inner: U = <_ as de::Deserialize<'de>>::deserialize(deserializer)?;
        Ok(Self(grug::Int::new(inner)))
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        assertor::*,
        grug::{Uint64, Uint128},
    };

    #[test]
    fn test_volume() {
        let volume = Volume::<Uint128>::from(Uint128::from(1000000000000000000));
        let serialized = serde_json::to_string(&volume).unwrap();
        let deserialized: Volume<Uint128> = serde_json::from_str(&serialized).unwrap();
        assert_that!(volume).is_equal_to(deserialized);

        let volume = Volume::<Uint64>::from(Uint64::from(1000000000000000000));
        let serialized = serde_json::to_string(&volume).unwrap();
        let deserialized: Volume<Uint64> = serde_json::from_str(&serialized).unwrap();
        assert_that!(volume).is_equal_to(deserialized);
    }
}
