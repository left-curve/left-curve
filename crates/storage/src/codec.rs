use {
    grug_types::{BorshExt, JsonExt, ProtoExt, StdResult},
    serde::{de::DeserializeOwned, ser::Serialize},
};

/// A marker that designates encoding/decoding schemes.
pub trait Codec<T> {
    fn encode(data: &T) -> StdResult<Vec<u8>>;

    fn decode(data: &[u8]) -> StdResult<T>;
}

// ----------------------------------- borsh -----------------------------------

/// Represents the Borsh encoding scheme.
pub struct Borsh;

impl<T> Codec<T> for Borsh
where
    T: BorshExt,
{
    fn encode(data: &T) -> StdResult<Vec<u8>> {
        data.to_borsh_vec()
    }

    fn decode(data: &[u8]) -> StdResult<T> {
        T::from_borsh_slice(data)
    }
}

// ----------------------------------- proto -----------------------------------

/// Represents the Protobuf encoding scheme.
pub struct Proto;

impl<T> Codec<T> for Proto
where
    T: ProtoExt,
{
    fn encode(data: &T) -> StdResult<Vec<u8>> {
        Ok(data.to_proto_vec())
    }

    fn decode(data: &[u8]) -> StdResult<T> {
        T::from_proto_slice(data)
    }
}

// -------------------------------- serde json ---------------------------------

/// Represents the JSON encoding scheme.
///
/// TODO: `Serde` is probably not a good naming, because serde library supports
/// more encoding schemes than just JSON. But for now I don't have a better idea
/// on how to name this.
pub struct Serde;

impl<T> Codec<T> for Serde
where
    T: Serialize + DeserializeOwned,
{
    fn encode(data: &T) -> StdResult<Vec<u8>> {
        data.to_json_vec()
    }

    fn decode(data: &[u8]) -> StdResult<T> {
        T::from_json_slice(data)
    }
}

// ------------------------------------ raw ------------------------------------

/// Represents raw bytes without encoding.
pub struct Raw;

impl Codec<Vec<u8>> for Raw {
    fn encode(data: &Vec<u8>) -> StdResult<Vec<u8>> {
        Ok(data.clone())
    }

    fn decode(data: &[u8]) -> StdResult<Vec<u8>> {
        Ok(data.to_vec())
    }
}
