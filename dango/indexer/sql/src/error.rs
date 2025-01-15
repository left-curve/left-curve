use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("sea_orm error: {0}")]
    SeaOrm(#[from] sea_orm::error::DbErr),
}
