use {
    crate::{Binary, StdError, StdResult},
    serde::{de::DeserializeOwned, ser::Serialize},
    std::any::type_name,
};

pub fn from_json<T>(bytes: impl AsRef<[u8]>) -> StdResult<T>
where
    T: DeserializeOwned,
{
    serde_json_wasm::from_slice(bytes.as_ref()).map_err(|reason| {
        StdError::Deserialize {
            ty: type_name::<T>(),
            reason,
        }
    })
}

pub fn to_json<T>(data: &T) -> StdResult<Binary>
where
    T: Serialize,
{
    serde_json_wasm::to_vec(data).map(Into::into).map_err(|reason| {
        StdError::Serialize {
            ty: type_name::<T>(),
            reason,
        }
    })
}
