use {
    crate::Binary,
    anyhow::anyhow,
    data_encoding::BASE64,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::any::type_name,
};

pub fn from_json<T>(bytes: impl AsRef<[u8]>) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    serde_json_wasm::from_slice(bytes.as_ref()).map_err(|err| {
        anyhow!(
            "Failed to deserialize from json! data: {}, reason: {}",
            BASE64.encode(bytes.as_ref()),
            err
        )
    })
}

pub fn to_json<T>(data: &T) -> anyhow::Result<Binary>
where
    T: Serialize,
{
    serde_json_wasm::to_vec(data).map(Into::into).map_err(|err| {
        anyhow!(
            "Failed to serialize to json! type: {}, reason: {}",
            type_name::<T>(),
            err,
        )
    })
}
