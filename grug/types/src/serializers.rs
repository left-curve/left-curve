use {
    crate::{Inner, Json, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    prost::Message,
    serde::{de::DeserializeOwned, ser::Serialize},
};

// ----------------------------------- json ------------------------------------

/// Represents a Rust value that can be serialized into JSON.
pub trait JsonSerExt: Sized {
    /// Serialize the Rust value into JSON bytes.
    fn to_json_vec(&self) -> StdResult<Vec<u8>>;

    /// Serialize the Rust value into JSON string.
    fn to_json_string(&self) -> StdResult<String>;

    /// Serialize the Rust value into pretty JSON string.
    fn to_json_string_pretty(&self) -> StdResult<String>;

    /// Serialize the Rust value into JSON value.
    fn to_json_value(&self) -> StdResult<Json>;
}

impl<T> JsonSerExt for T
where
    T: Serialize,
{
    fn to_json_vec(&self) -> StdResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(|err| StdError::serialize::<T, _>("json", err))
    }

    fn to_json_string(&self) -> StdResult<String> {
        serde_json::to_string(self).map_err(|err| StdError::serialize::<T, _>("json", err))
    }

    fn to_json_string_pretty(&self) -> StdResult<String> {
        serde_json::to_string_pretty(self).map_err(|err| StdError::serialize::<T, _>("json", err))
    }

    fn to_json_value(&self) -> StdResult<Json> {
        serde_json::to_value(self)
            .map(Json::from_inner)
            .map_err(|err| StdError::serialize::<T, _>("json", err))
    }
}

/// Represents raw JSON data that can be deserialized into a Rust value.
pub trait JsonDeExt {
    /// Deserialize the raw data into a Rust value.
    fn deserialize_json<D>(self) -> StdResult<D>
    where
        D: DeserializeOwned;
}

impl<T> JsonDeExt for &T
where
    T: AsRef<[u8]>,
{
    fn deserialize_json<D>(self) -> StdResult<D>
    where
        D: DeserializeOwned,
    {
        serde_json::from_slice(self.as_ref())
            .map_err(|err| StdError::deserialize::<D, _>("json", err))
    }
}

impl JsonDeExt for Json {
    fn deserialize_json<D>(self) -> StdResult<D>
    where
        D: DeserializeOwned,
    {
        serde_json::from_value(self.into_inner())
            .map_err(|err| StdError::deserialize::<D, _>("json", err))
    }
}

// ----------------------------------- borsh -----------------------------------

/// Represents a Rust value that can be serialized into raw bytes using the
/// [Borsh](https://github.com/near/borsh) encoding.
pub trait BorshSerExt: Sized {
    /// Serialize the Rust value into Borsh bytes.
    fn to_borsh_vec(&self) -> StdResult<Vec<u8>>;
}

impl<T> BorshSerExt for T
where
    T: BorshSerialize,
{
    fn to_borsh_vec(&self) -> StdResult<Vec<u8>> {
        borsh::to_vec(self).map_err(|err| StdError::serialize::<T, _>("borsh", err))
    }
}

/// Represents raw bytes that can be deserialized into a Rust value using the
/// [Borsh](https://github.com/near/borsh) encoding.
pub trait BorshDeExt {
    fn deserialize_borsh<D>(self) -> StdResult<D>
    where
        D: BorshDeserialize;
}

impl<T> BorshDeExt for &T
where
    T: AsRef<[u8]>,
{
    fn deserialize_borsh<D>(self) -> StdResult<D>
    where
        D: BorshDeserialize,
    {
        borsh::from_slice(self.as_ref()).map_err(|err| StdError::deserialize::<D, _>("borsh", err))
    }
}

// --------------------------------- protobuf ----------------------------------

/// Represents a Rust value that can be serialized into raw bytes using the
/// Protobuf encoding.
pub trait ProtoSerExt: Sized {
    /// Serialize the Rust value into Protobuf bytes.
    fn to_proto_vec(&self) -> Vec<u8>;
}

impl<T> ProtoSerExt for T
where
    T: Message + Default,
{
    fn to_proto_vec(&self) -> Vec<u8> {
        self.encode_to_vec()
    }
}

/// Represents raw bytes that can be deserialized into a Rust value using the
/// Protobuf encoding.
pub trait ProtoDeExt {
    fn deserialize_proto<D>(self) -> StdResult<D>
    where
        D: Message + Default;
}

impl<T> ProtoDeExt for &T
where
    T: AsRef<[u8]>,
{
    fn deserialize_proto<D>(self) -> StdResult<D>
    where
        D: Message + Default,
    {
        D::decode(self.as_ref()).map_err(|err| StdError::deserialize::<D, _>("protobuf", err))
    }
}
