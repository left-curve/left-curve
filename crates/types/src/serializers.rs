use {
    crate::{Json, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    prost::Message,
    serde::{de::DeserializeOwned, ser::Serialize},
};

/// Deserialize a JSON value into Rust value of a given type `T`.
pub fn from_json_value<T>(json: Json) -> StdResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(json).map_err(|err| StdError::deserialize::<T, _>("json", err))
}

/// Serialize a Rust value into JSON value.
pub fn to_json_value<T>(data: &T) -> StdResult<Json>
where
    T: Serialize,
{
    serde_json::to_value(data).map_err(|err| StdError::serialize::<T, _>("json", err))
}

/// Deserialize a slice of bytes into Rust value of a given type `T` using the
/// JSON encoding scheme.
pub fn from_json_slice<B, T>(bytes: B) -> StdResult<T>
where
    B: AsRef<[u8]>,
    T: DeserializeOwned,
{
    serde_json::from_slice(bytes.as_ref()).map_err(|err| StdError::deserialize::<T, _>("json", err))
}

/// Serialize a Rust value into bytes using the JSON encoding scheme.
pub fn to_json_vec<T>(data: &T) -> StdResult<Vec<u8>>
where
    T: Serialize,
{
    serde_json::to_vec(data).map_err(|err| StdError::serialize::<T, _>("json", err))
}

/// Deserialize a JSON string into Rust value of a given type `T`.
pub fn from_json_str<S, T>(string: S) -> StdResult<T>
where
    S: AsRef<str>,
    T: DeserializeOwned,
{
    serde_json::from_str(string.as_ref()).map_err(|err| StdError::deserialize::<T, _>("json", err))
}

/// Serialize a Rust value into a JSON string.
pub fn to_json_string<T>(data: &T) -> StdResult<String>
where
    T: Serialize,
{
    serde_json::to_string(data).map_err(|err| StdError::serialize::<T, _>("json", err))
}

/// Serialize a Rust value into a a pretty JSON string.
pub fn to_json_string_pretty<T>(data: &T) -> StdResult<String>
where
    T: Serialize,
{
    serde_json::to_string_pretty(data).map_err(|err| StdError::serialize::<T, _>("json", err))
}

/// Deserialize a slice of bytes into Rust value of a given type `T` using the
/// [Borsh](https://crates.io/crates/borsh) encoding scheme.
pub fn from_borsh_slice<B, T>(bytes: B) -> StdResult<T>
where
    B: AsRef<[u8]>,
    T: BorshDeserialize,
{
    borsh::from_slice(bytes.as_ref()).map_err(|err| StdError::deserialize::<T, _>("borsh", err))
}

/// Serialize a Rust value into bytes using the [Borsh](https://crates.io/crates/borsh)
/// encoding scheme.
pub fn to_borsh_vec<T>(data: &T) -> StdResult<Vec<u8>>
where
    T: BorshSerialize,
{
    borsh::to_vec(data).map_err(|err| StdError::serialize::<T, _>("borsh", err))
}

/// Deserialize a slice of bytes into Rust value of a given type `T` using the
/// Protobuf encoding scheme.
pub fn from_proto_slice<B, T>(bytes: B) -> StdResult<T>
where
    B: AsRef<[u8]>,
    T: Message + Default,
{
    T::decode(bytes.as_ref()).map_err(|err| StdError::deserialize::<T, _>("protobuf", err))
}

/// Serialize a Rust value into bytes using the Protobuf encoding scheme.
pub fn to_proto_vec<T>(data: &T) -> Vec<u8>
where
    T: Message,
{
    data.encode_to_vec()
}
