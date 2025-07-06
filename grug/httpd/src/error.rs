use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("GraphQL error: {0}")]
    GraphQL(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<async_graphql::Error> for Error {
    fn from(e: async_graphql::Error) -> Self {
        Error::GraphQL(format!("{e:?}"))
    }
}
