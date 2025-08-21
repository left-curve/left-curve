use {indexer_sql::pubsub::error::PubSubError, thiserror::Error};

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Std(#[from] grug::StdError),

    #[error(transparent)]
    Math(#[from] grug::MathError),

    #[error(transparent)]
    Clickhouse(#[from] clickhouse::error::Error),

    #[error("missing block or block outcome")]
    MissingBlockOrBlockOutcome,

    #[error("candle timeout")]
    CandleTimeout,

    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    PubSub(#[from] PubSubError),
}

impl From<IndexerError> for grug_app::IndexerError {
    fn from(error: IndexerError) -> Self {
        match error {
            IndexerError::Clickhouse(error) => grug_app::IndexerError::Database(error.to_string()),
            IndexerError::Io(error) => grug_app::IndexerError::Io(error.to_string()),
            err => grug_app::IndexerError::Hook(err.to_string()),
        }
    }
}

pub type Result<T> = core::result::Result<T, IndexerError>;
