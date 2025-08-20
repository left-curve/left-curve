use {indexer_sql::pubsub::error::PubSubError, thiserror::Error};

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StdError(#[from] grug::StdError),

    #[error(transparent)]
    ClickhouseError(#[from] clickhouse::error::Error),

    #[error("missing block or block outcome")]
    MissingBlockOrBlockOutcome,

    #[error(transparent)]
    GrugMathError(#[from] grug::MathError),

    #[error("candle timeout")]
    CandleTimeout,

    #[error(transparent)]
    ChronoParseError(#[from] chrono::ParseError),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    PubSubError(#[from] PubSubError),
}

impl From<IndexerError> for grug_app::IndexerError {
    fn from(error: IndexerError) -> Self {
        match error {
            IndexerError::ClickhouseError(error) => {
                grug_app::IndexerError::Database(error.to_string())
            },
            IndexerError::Io(error) => grug_app::IndexerError::Io(error.to_string()),
            err => grug_app::IndexerError::Hook(err.to_string()),
        }
    }
}

pub type Result<T> = core::result::Result<T, IndexerError>;
