use {error_backtrace::Backtraceable, indexer_sql::pubsub::error::PubSubError};

#[error_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error(transparent)]
    #[backtrace(new)]
    Io(std::io::Error),

    #[error(transparent)]
    Std(grug::StdError),

    #[error(transparent)]
    Math(grug::MathError),

    #[error(transparent)]
    #[backtrace(new)]
    Clickhouse(clickhouse::error::Error),

    #[error("missing block or block outcome")]
    MissingBlockOrBlockOutcome,

    #[error("candle timeout")]
    CandleTimeout,

    #[error(transparent)]
    #[backtrace(new)]
    ChronoParse(chrono::ParseError),

    #[error(transparent)]
    #[backtrace(new)]
    SerdeJson(serde_json::Error),

    #[error(transparent)]
    PubSub(PubSubError),
}

macro_rules! parse_error {
    ($variant:ident, $e:expr) => {
        grug_app::IndexerError::$variant {
            error: $e.to_string(),
            backtrace: $e.backtrace,
        }
    };
}

impl From<IndexerError> for grug_app::IndexerError {
    fn from(error: IndexerError) -> Self {
        match error {
            IndexerError::Clickhouse(error) => parse_error!(Database, error),
            IndexerError::Io(error) => parse_error!(Io, error),
            err => {
                let err = err.into_generic_backtraced_error();
                parse_error!(Hook, err)
            },
        }
    }
}

pub type Result<T> = core::result::Result<T, IndexerError>;
