use {dango_backtrace::Backtraceable, dango_indexer_sql::pubsub::error::PubSubError};

#[dango_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error(transparent)]
    #[backtrace(new)]
    Io(std::io::Error),

    #[error(transparent)]
    Std(dango_primitives::StdError),

    #[error(transparent)]
    Math(dango_math::MathError),

    #[error(transparent)]
    #[backtrace(new)]
    Clickhouse(clickhouse::error::Error),

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
        dango_app::IndexerError::$variant {
            error: $e.to_string(),
            backtrace: $e.backtrace,
        }
    };
}

impl From<IndexerError> for dango_app::IndexerError {
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
