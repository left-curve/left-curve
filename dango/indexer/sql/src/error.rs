use {indexer_sql::pubsub::error::PubSubError, thiserror::Error};

#[derive(Debug, Error)]
pub enum Error {
    #[error("sea_orm error: {0}")]
    SeaOrm(#[from] sea_orm::error::DbErr),

    #[error("wrong event type")]
    WrongEventType,

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("grug error: {0}")]
    Std(#[from] grug::StdError),

    #[error(transparent)]
    PubSub(#[from] PubSubError),

    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}

impl From<Error> for grug_app::IndexerError {
    fn from(error: Error) -> Self {
        grug_app::IndexerError::Hook(error.to_string())
    }
}
