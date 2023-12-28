use {
    anyhow::Context,
    data_encoding::BASE64,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::any::type_name,
};

pub fn from_json<T>(bytes: impl AsRef<[u8]>) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    serde_json_wasm::from_slice(bytes.as_ref())
        .with_context(|| format!("Failed to deserialize from json! data: {}", BASE64.encode(bytes.as_ref())))
}

pub fn to_json<T>(data: &T) -> anyhow::Result<Vec<u8>>
where
    T: Serialize,
{
    serde_json_wasm::to_vec(data)
        .with_context(|| format!("Failed to serialize to json! type: {}", type_name::<T>()))
}
