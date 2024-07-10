use {
    crate::{Json, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    prost::Message,
    serde::{de::DeserializeOwned, ser::Serialize},
    serde_json::value::Index,
};

/// Deserialize a JSON value into Rust value of a given type `T`.
pub fn from_json_value<T>(json: Json) -> StdResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(json).map_err(StdError::deserialize::<T>)
}

pub fn from_json_key_value<T, I>(json: Json, key: I) -> StdResult<T>
where
    T: DeserializeOwned,
    I: Index,
{
    let value = json
        .get(key)
        .cloned()
        .ok_or(StdError::generic_err("Key not found"))?;
    from_json_value(value)
}
/// Serialize a Rust value into JSON value.
pub fn to_json_value<T>(data: &T) -> StdResult<Json>
where
    T: Serialize,
{
    serde_json::to_value(data)
        .map(Into::into)
        .map_err(StdError::serialize::<T>)
}

/// Deserialize a slice of bytes into Rust value of a given type `T` using the
/// JSON encoding scheme.
pub fn from_json_slice<T>(bytes: impl AsRef<[u8]>) -> StdResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(bytes.as_ref()).map_err(StdError::deserialize::<T>)
}

/// Serialize a Rust value into bytes using the JSON encoding scheme.
pub fn to_json_vec<T>(data: &T) -> StdResult<Vec<u8>>
where
    T: Serialize,
{
    serde_json::to_vec(data).map_err(StdError::serialize::<T>)
}

/// Deserialize a slice of bytes into Rust value of a given type `T` using the
/// [Borsh](https://crates.io/crates/borsh) encoding scheme.
pub fn from_borsh_slice<T>(bytes: impl AsRef<[u8]>) -> StdResult<T>
where
    T: BorshDeserialize,
{
    borsh::from_slice(bytes.as_ref()).map_err(StdError::deserialize::<T>)
}

/// Serialize a Rust value into bytes using the [Borsh](https://crates.io/crates/borsh)
/// encoding scheme.
pub fn to_borsh_vec<T>(data: &T) -> StdResult<Vec<u8>>
where
    T: BorshSerialize,
{
    borsh::to_vec(data).map_err(StdError::serialize::<T>)
}

/// Deserialize a slice of bytes into Rust value of a given type `T` using the
/// Protobuf encoding scheme.
pub fn from_proto_slice<T>(bytes: impl AsRef<[u8]>) -> StdResult<T>
where
    T: Message + Default,
{
    T::decode(bytes.as_ref()).map_err(StdError::deserialize::<T>)
}

/// Serialize a Rust value into bytes using the Protobuf encoding scheme.
pub fn to_proto_vec<T>(data: &T) -> Vec<u8>
where
    T: Message,
{
    data.encode_to_vec()
}


