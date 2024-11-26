#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sea_orm error: {0}")]
    SeaOrm(#[from] sea_orm::error::DbErr),

    #[error("anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("JoinError: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

pub type Result<T> = core::result::Result<T, Error>;

#[macro_export]
macro_rules! bail {
    ($variant:path, $msg:expr) => {
        return Err($variant($msg.into()).into());
    };
    ($($arg:tt)*) => {
        return Err($crate::error::Error::Anyhow(anyhow::anyhow!(format!($($arg)*))));
    };
}
