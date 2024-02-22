use {
    crate::{Binary, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de::DeserializeOwned, ser::Serialize},
};

pub fn from_json<T>(bytes: impl AsRef<[u8]>) -> StdResult<T>
where
    T: DeserializeOwned,
{
    serde_json_wasm::from_slice(bytes.as_ref()).map_err(StdError::deserialize::<T>)
}

pub fn to_json<T>(data: &T) -> StdResult<Binary>
where
    T: Serialize,
{
    serde_json_wasm::to_vec(data).map(Into::into).map_err(StdError::serialize::<T>)
}

pub fn from_borsh<T>(bytes: impl AsRef<[u8]>) -> StdResult<T>
where
    T: BorshDeserialize,
{
    borsh::from_slice(bytes.as_ref()).map_err(StdError::deserialize::<T>)
}

pub fn to_borsh<T>(data: &T) -> StdResult<Binary>
where
    T: BorshSerialize,
{
    borsh::to_vec(data).map(Into::into).map_err(StdError::serialize::<T>)
}
