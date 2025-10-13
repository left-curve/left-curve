use {error_backtrace::Backtraceable, indexer_sql::pubsub::error::PubSubError};

#[error_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sea_orm error: {0}")]
    #[backtrace(new)]
    SeaOrm(sea_orm::error::DbErr),

    #[error("wrong event type")]
    WrongEventType,

    #[error("serde error: {0}")]
    #[backtrace(new)]
    Serde(serde_json::Error),

    #[error("grug error: {0}")]
    Std(grug::StdError),

    #[error(transparent)]
    PubSub(PubSubError),

    #[error(transparent)]
    #[backtrace(new)]
    Join(tokio::task::JoinError),
}

impl From<Error> for grug_app::IndexerError {
    fn from(error: Error) -> Self {
        let bt = error.backtrace();
        grug_app::IndexerError::Hook {
            error: error.to_string(),
            backtrace: bt,
        }
    }
}
