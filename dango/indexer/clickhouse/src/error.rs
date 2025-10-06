use {grug::Backtraceable, indexer_sql::pubsub::error::PubSubError};

#[grug_macros::backtrace]
pub enum IndexerError {
    #[error(transparent)]
    #[backtrace(new)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Std(#[from] grug::StdError),

    #[error(transparent)]
    Math(#[from] grug::MathError),

    #[error(transparent)]
    #[backtrace(new)]
    Clickhouse(#[from] clickhouse::error::Error),

    #[error("missing block or block outcome")]
    MissingBlockOrBlockOutcome,

    #[error("candle timeout")]
    CandleTimeout,

    #[error(transparent)]
    #[backtrace(new)]
    ChronoParse(#[from] chrono::ParseError),

    #[error(transparent)]
    #[backtrace(new)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    PubSub(#[from] PubSubError),
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
