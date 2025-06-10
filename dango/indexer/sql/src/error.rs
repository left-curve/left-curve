use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("sea_orm error: {0}")]
    SeaOrm(#[from] sea_orm::error::DbErr),

    #[error("wrong event type")]
    WrongEventType,

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("grug std error: {0}")]
    Std(#[from] grug::StdError),
}
