use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{
        from_borsh_slice, from_json_slice, from_proto_slice, to_borsh_vec, to_json_vec,
        to_proto_vec, StdResult,
    },
    prost::Message,
    serde::{de::DeserializeOwned, ser::Serialize},
};

/// A marker that designates encoding/decoding schemes.
pub trait Codec<T> {
    fn encode(data: &T) -> StdResult<Vec<u8>>;

    fn decode(data: &[u8]) -> StdResult<T>;
}

/// Represents the Borsh encoding scheme.
pub struct Borsh;

impl<T> Codec<T> for Borsh
where
    T: BorshSerialize + BorshDeserialize,
{
    fn encode(data: &T) -> StdResult<Vec<u8>> {
        to_borsh_vec(&data)
    }

    fn decode(data: &[u8]) -> StdResult<T> {
        from_borsh_slice(data)
    }
}

/// Represents the Protobuf encoding scheme.
pub struct Proto;

impl<T> Codec<T> for Proto
where
    T: Message + Default,
{
    fn encode(data: &T) -> StdResult<Vec<u8>> {
        Ok(to_proto_vec(data))
    }

    fn decode(data: &[u8]) -> StdResult<T> {
        from_proto_slice(data)
    }
}

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
        to_json_vec(data)
    }

    fn decode(data: &[u8]) -> StdResult<T> {
        from_json_slice(data)
    }
}

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
