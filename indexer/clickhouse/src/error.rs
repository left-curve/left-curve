use grug::Backtraceable;

#[grug_macros::backtrace]
pub enum IndexerError {
    #[error(transparent)]
    #[backtrace(new)]
    Io(std::io::Error),

    #[error(transparent)]
    StdError(grug::StdError),

    #[error(transparent)]
    #[backtrace(new)]
    ClickhouseError(clickhouse::error::Error),

    #[error("missing block or block outcome")]
    MissingBlockOrBlockOutcome,

    #[error(transparent)]
    GrugMathError(grug::MathError),
}

impl From<IndexerError> for grug_app::IndexerError {
    fn from(error: IndexerError) -> Self {
        match error {
            IndexerError::ClickhouseError(error) => grug_app::IndexerError::Database {
                error: error.to_string(),
                backtrace: error.backtrace(),
            },
            IndexerError::Io(error) => grug_app::IndexerError::Io {
                error: error.to_string(),
                backtrace: error.backtrace(),
            },
            err => grug_app::IndexerError::Hook {
                error: err.to_string(),
                backtrace: err.backtrace(),
            },
        }
    }
}

pub type Result<T> = core::result::Result<T, IndexerError>;
