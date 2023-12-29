use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Response {
    // TODO: add stuff
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ContractResult {
    Ok(Response),
    Err(String),
}

impl<E> From<Result<Response, E>> for ContractResult
where
    E: ToString,
{
    fn from(res: Result<Response, E>) -> Self {
        match res {
            Result::Ok(resp) => Self::Ok(resp),
            Result::Err(err) => Self::Err(err.to_string()),
        }
    }
}
