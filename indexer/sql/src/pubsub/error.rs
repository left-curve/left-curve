#[grug_macros::backtrace]
pub enum PubSubError {
    #[error("Failed to publish item")]
    PublishFailed,

    #[error(transparent)]
    #[backtrace(new)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    #[backtrace(new)]
    SerdeJson(#[from] serde_json::Error),
}

pub type Result<T> = core::result::Result<T, PubSubError>;
