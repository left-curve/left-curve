use {cw_std::StdError, thiserror::Error};

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    RocksDb(#[from] rocksdb::Error),
}

pub type DbResult<T> = std::result::Result<T, DbError>;
