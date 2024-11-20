#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sea_orm error: {0}")]
    SeaOrm(#[from] sea_orm::error::DbErr),
}

pub type Result<T> = core::result::Result<T, Error>;
