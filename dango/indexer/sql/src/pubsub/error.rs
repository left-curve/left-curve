#[dango_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum PubSubError {
    #[error(transparent)]
    #[backtrace(new)]
    Sqlx(sqlx::Error),

    #[error(transparent)]
    #[backtrace(new)]
    SerdeJson(serde_json::Error),
}

pub type Result<T> = core::result::Result<T, PubSubError>;
