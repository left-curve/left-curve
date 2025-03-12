use {thiserror::Error, url::ParseError};
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    CannotClone(#[from] reqwest_eventsource::CannotCloneRequestError),

    #[error("data not found! type: {ty}, storage key: {key}")]
    DataNotFound { ty: &'static str, key: String },

    #[error(transparent)]
    ParseError(#[from] ParseError),
}
