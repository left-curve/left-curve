use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ContractResult<T> {
    Ok(T),
    Err(String),
}

impl<T, E> From<Result<T, E>> for ContractResult<T>
where
    E: ToString,
{
    fn from(res: Result<T, E>) -> Self {
        match res {
            Result::Ok(resp) => Self::Ok(resp),
            Result::Err(err) => Self::Err(err.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Response {
    // TODO: add stuff
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }
}
