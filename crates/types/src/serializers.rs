use {
    crate::{Json, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    prost::Message,
    serde::{de::DeserializeOwned, ser::Serialize},
};

// ----------------------------------- json ------------------------------------

pub trait JsonExt: Sized {
    /// Deserialize a slice of JSON bytes into Rust value.
    fn from_json_slice<B>(bytes: B) -> StdResult<Self>
    where
        B: AsRef<[u8]>;

    /// Deserialize a JSON string into Rust value.
    fn from_json_str<S>(string: S) -> StdResult<Self>
    where
        S: AsRef<str>;

    /// Deserialize a JSON value into Rust value.
    fn from_json_value(value: Json) -> StdResult<Self>;

    /// Serialize the Rust value into JSON bytes.
    fn to_json_vec(&self) -> StdResult<Vec<u8>>;

    /// Serialize the Rust value into JSON string.
    fn to_json_string(&self) -> StdResult<String>;

    /// Serialize the Rust value into pretty JSON string.
    fn to_json_string_pretty(&self) -> StdResult<String>;

    /// Serialize the Rust value into JSON value.
    fn to_json_value(&self) -> StdResult<Json>;
}

impl<T> JsonExt for T
where
    T: Serialize + DeserializeOwned,
{
    fn from_json_slice<B>(bytes: B) -> StdResult<Self>
    where
        B: AsRef<[u8]>,
    {
        serde_json::from_slice(bytes.as_ref())
            .map_err(|err| StdError::deserialize::<T, _>("json", err))
    }

    fn from_json_str<S>(string: S) -> StdResult<Self>
    where
        S: AsRef<str>,
    {
        serde_json::from_str(string.as_ref())
            .map_err(|err| StdError::deserialize::<T, _>("json", err))
    }

    fn from_json_value(value: Json) -> StdResult<Self> {
        serde_json::from_value(value).map_err(|err| StdError::deserialize::<T, _>("json", err))
    }

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
        serde_json::to_value(self).map_err(|err| StdError::serialize::<T, _>("json", err))
    }
}

// ----------------------------------- borsh -----------------------------------

pub trait BorshExt: Sized {
    /// Deserialize a slice of Borsh bytes into Rust value.
    fn from_borsh_slice<B>(bytes: B) -> StdResult<Self>
    where
        B: AsRef<[u8]>;

    /// Serialize the Rust value into Borsh bytes.
    fn to_borsh_vec(&self) -> StdResult<Vec<u8>>;
}

impl<T> BorshExt for T
where
    T: BorshSerialize + BorshDeserialize,
{
    fn from_borsh_slice<B>(bytes: B) -> StdResult<Self>
    where
        B: AsRef<[u8]>,
    {
        borsh::from_slice(bytes.as_ref()).map_err(|err| StdError::deserialize::<T, _>("borsh", err))
    }

    fn to_borsh_vec(&self) -> StdResult<Vec<u8>> {
        borsh::to_vec(self).map_err(|err| StdError::serialize::<T, _>("borsh", err))
    }
}

// --------------------------------- protobuf ----------------------------------

pub trait ProtoExt: Sized {
    /// Deserialize a slice of Protobuf bytes into Rust value.
    fn from_proto_slice<B>(bytes: B) -> StdResult<Self>
    where
        B: AsRef<[u8]>;

    /// Serialize the Rust value into Protobuf bytes.
    fn to_proto_vec(&self) -> Vec<u8>;
}

impl<T> ProtoExt for T
where
    T: Message + Default,
{
    fn from_proto_slice<B>(bytes: B) -> StdResult<Self>
    where
        B: AsRef<[u8]>,
    {
        T::decode(bytes.as_ref()).map_err(|err| StdError::deserialize::<T, _>("protobuf", err))
    }

    fn to_proto_vec(&self) -> Vec<u8> {
        self.encode_to_vec()
    }
}
