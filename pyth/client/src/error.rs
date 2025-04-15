use {thiserror::Error, url::ParseError};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    ParseErrorUrl(#[from] ParseError),
}
