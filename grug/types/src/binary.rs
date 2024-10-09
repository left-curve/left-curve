use {
    crate::{StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    data_encoding::BASE64,
    grug_math::Inner,
    serde::{de, ser},
    std::{
        fmt::{self},
        marker::PhantomData,
        ops::{Deref, DerefMut},
    },
};

pub type Binary = B64<Vec<u8>>;
pub type ByteArray<const N: usize> = B64<[u8; N]>;

#[derive(
    BorshSerialize, BorshDeserialize, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct B64<T>(T);

impl<T> B64<T>
where
    T: Default,
{
    pub fn empty() -> Self {
        Self(T::default())
    }
}

impl<T> Inner for B64<T> {
    type U = T;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl<T> AsRef<[u8]> for B64<T>
where
    T: AsRef<[u8]>,
{
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<T> AsMut<[u8]> for B64<T>
where
    T: AsMut<[u8]>,
{
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl<T> Deref for B64<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for B64<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> fmt::Display for B64<T>
where
    T: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", BASE64.encode(self.0.as_ref()))
    }
}

impl<T> fmt::Debug for B64<T>
where
    T: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Binary({})", BASE64.encode(self.0.as_ref()))
    }
}

impl<T> ser::Serialize for B64<T>
where
    T: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&BASE64.encode(self.0.as_ref()))
    }
}

impl<'de, T> de::Deserialize<'de> for B64<T>
where
    T: TryFrom<Vec<u8>>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(BinaryVisitor::<T>(PhantomData))
    }
}

struct BinaryVisitor<T>(PhantomData<T>);

impl<'de, T> de::Visitor<'de> for BinaryVisitor<T>
where
    T: TryFrom<Vec<u8>>,
{
    type Value = B64<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A base64 encoded string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        BASE64
            .decode(v.as_bytes())
            .map_err(|err| E::custom(format!("Invalid base64: {err}")))?
            .try_into_b64()
            .map_err(|_| E::custom(StdError::invalid_conversion::<Vec<u8>, T>()))
    }
}

// ----------------------------- conversions -----------------------------

/// Conversion trait for types that can be converted into a `B64` type.
/// Is not possible to implement `From/Into` trait for generics because of
/// core conflicting implementations.
pub trait TryIntoB64<T> {
    fn try_into_b64(self) -> StdResult<B64<T>>;
}

impl<T, U> TryIntoB64<T> for U
where
    T: TryFrom<U>,
{
    fn try_into_b64(self) -> StdResult<B64<T>> {
        T::try_from(self)
            .map_err(|_| StdError::invalid_conversion::<Self, T>())
            .map(B64)
    }
}

impl<T, const N: usize> From<[u8; N]> for B64<T>
where
    T: From<[u8; N]>,
{
    fn from(array: [u8; N]) -> Self {
        Self(T::from(array))
    }
}

impl<'a, T> From<&'a [u8]> for B64<T>
where
    T: From<&'a [u8]>,
{
    fn from(slice: &'a [u8]) -> Self {
        Self(T::from(slice))
    }
}

impl<T> From<Vec<u8>> for B64<T>
where
    T: From<Vec<u8>>,
{
    fn from(vec: Vec<u8>) -> Self {
        Self(T::from(vec))
    }
}

impl<T> From<String> for B64<T>
where
    T: From<Vec<u8>>,
{
    fn from(string: String) -> Self {
        Self(T::from(string.into_bytes()))
    }
}

impl<T> From<&str> for B64<T>
where
    T: From<Vec<u8>>,
{
    fn from(s: &str) -> Self {
        Self(T::from(s.as_bytes().to_vec()))
    }
}

// -------------------------------- tests --------------------------------

#[cfg(test)]
mod tests {
    use {
        super::Binary,
        crate::{ByteArray, JsonDeExt, JsonSerExt},
    };

    #[test]
    fn binary() {
        let binary = Binary::from(vec![1, 2, 3]);
        let se = binary.to_json_string().unwrap();
        let de: Binary = se.deserialize_json().unwrap();
        assert_eq!(binary, de);
    }

    #[test]
    fn byte_array() {
        let fixed = ByteArray::from([1, 2, 3]);

        let se = fixed.to_json_string().unwrap();
        let de: ByteArray<3> = se.deserialize_json().unwrap();

        assert_eq!(fixed, de);

        // not working cause the array length is different
        serde_json::from_str::<ByteArray<4>>(&se).unwrap_err();
    }
}
