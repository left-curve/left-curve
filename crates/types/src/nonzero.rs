use {
    crate::{Number, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de, de::Error, Serialize},
    std::{
        fmt,
        fmt::Display,
        io,
        ops::{Deref, DerefMut},
    },
};

/// A wrapper over a number that ensures it is non-zero.
#[derive(Serialize, BorshSerialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonZero<T>(pub(crate) T);

impl<T> NonZero<T>
where
    T: Number,
{
    /// Attempt to create a new non-zero wrapper. Error if a zero is provided.
    pub fn new(inner: T) -> StdResult<Self> {
        if inner.is_zero() {
            return Err(StdError::zero_value::<Self>());
        }

        Ok(Self(inner))
    }
}

impl<T> NonZero<T> {
    /// Consume the wrapper, return the wrapped number.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> AsRef<T> for NonZero<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for NonZero<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Deref for NonZero<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NonZero<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Display for NonZero<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de, T> de::Deserialize<'de> for NonZero<T>
where
    T: Number + de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let inner = T::deserialize(deserializer)?;

        // We assert the number is non-zero here with `NonZero::new`.
        NonZero::new(inner).map_err(D::Error::custom)
    }
}

impl<T> BorshDeserialize for NonZero<T>
where
    T: Number + BorshDeserialize,
{
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        let inner = borsh::from_reader(reader)?;

        // We assert the number is non-zero here with `NonZero::new`.
        NonZero::new(inner).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{BorshDeExt, BorshSerExt, JsonDeExt, NonZero, NumberConst, StdError, Uint128};

    // The expect error is a `StdError::Deserialize` where the `reason` is a
    // `StdError::ZeroValue`.
    fn assert_is_non_zero_err<T>(err: StdError) {
        assert!(matches!(
            err,
            StdError::Deserialize { reason, .. } if reason == StdError::zero_value::<T>().to_string()
        ));
    }

    #[test]
    fn deserializing_from_json() {
        let res = "123".deserialize_json::<NonZero<u32>>().unwrap();
        assert_eq!(res, NonZero(123));

        let err = "0".deserialize_json::<NonZero<u32>>().unwrap_err();
        assert_is_non_zero_err::<NonZero<u32>>(err);

        let res = "\"123\"".deserialize_json::<NonZero<Uint128>>().unwrap();
        assert_eq!(res, NonZero(Uint128::new(123)));

        let err = "\"0\"".deserialize_json::<NonZero<Uint128>>().unwrap_err();
        assert_is_non_zero_err::<NonZero<Uint128>>(err);
    }

    #[test]
    fn deserialize_from_borsh() {
        let good = NonZero(Uint128::new(123)).to_borsh_vec().unwrap();
        let res = good.deserialize_borsh::<NonZero<Uint128>>().unwrap();
        assert_eq!(res, NonZero(Uint128::new(123)));

        // Construct an illegal `NonZero` with a zero inside.
        // This is only possible here because the inner value is `pub(crate)`.
        let bad = NonZero(Uint128::ZERO).to_borsh_vec().unwrap();
        let err = bad.deserialize_borsh::<NonZero<Uint128>>().unwrap_err();
        assert_is_non_zero_err::<NonZero<Uint128>>(err);
    }
}
