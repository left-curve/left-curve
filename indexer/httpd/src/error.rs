use {indexer_sql::error::IndexerError, sea_orm::sqlx, std::io};

#[grug_macros::backtrace]
pub enum Error {
    #[error(transparent)]
    #[backtrace(new)]
    Io(io::Error),

    #[error(transparent)]
    #[backtrace(new)]
    SeaOrm(sea_orm::DbErr),

    #[error(transparent)]
    #[backtrace(new)]
    Sqlx(sqlx::Error),

    #[error(transparent)]
    Indexer(IndexerError),
}
