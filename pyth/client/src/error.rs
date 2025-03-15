use {thiserror::Error, url::ParseError};
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    CannotClone(#[from] reqwest_eventsource::CannotCloneRequestError),

    #[error(transparent)]
    ParseErrorUrl(#[from] ParseError),
}
