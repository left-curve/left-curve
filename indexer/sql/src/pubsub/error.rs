#[error_backtrace::backtrace]
pub enum PubSubError {
    #[error("Failed to publish item")]
    PublishFailed,

    #[error(transparent)]
    #[backtrace(new)]
    Sqlx(sqlx::Error),

    #[error(transparent)]
    #[backtrace(new)]
    SerdeJson(serde_json::Error),
}

pub type Result<T> = core::result::Result<T, PubSubError>;
