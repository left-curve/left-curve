use {
    serde_json_wasm::{de::Error as DeserializeError, ser::Error as SerializeError},
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum StdError {
    #[error("Generic error: {0}")]
    Generic(String),

    #[error("Failed to serialize into json! type: {ty}, reason: {reason}")]
    Serialize {
        ty:     &'static str,
        reason: SerializeError,
    },

    #[error("Failed to deserialize from json! type: {ty}, reason: {reason}")]
    Deserialize {
        ty:     &'static str,
        reason: DeserializeError,
    },
}

pub type StdResult<T> = std::result::Result<T, StdError>;
