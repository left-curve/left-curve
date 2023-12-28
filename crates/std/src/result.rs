use {
    serde::{Serialize, Deserialize},
    schemars::JsonSchema,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Response {}

impl Response {
    pub fn new() -> Self {
        Self {}
    }
}
