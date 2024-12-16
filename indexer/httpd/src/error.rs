use {std::io, thiserror::Error};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    SeaOrm(#[from] sea_orm::DbErr),
}
