#[derive(Debug, thiserror::Error)]
pub enum PubSubError {
    #[error("Failed to publish item")]
    PublishFailed,

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

pub type Result<T> = core::result::Result<T, PubSubError>;
