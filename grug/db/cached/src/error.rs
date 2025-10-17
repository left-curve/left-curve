use {error_backtrace::Backtraceable, std::fmt};

#[error_backtrace::backtrace]
#[derive(Debug, Clone, thiserror::Error)]
pub enum DbError<B: fmt::Display + Backtraceable> {
    #[error(transparent)]
    Base(B),

    #[error("version not in memory: {version}")]
    VersionNotInMemory { version: u64 },

    #[error("state proof is not supported")]
    ProofUnsupported,

    #[error("next pending not set")]
    NextPendingNotSet,

    #[error("next version not set")]
    NextVersionNotSet,
}

pub type DbResult<T, B> = Result<T, DbError<B>>;
