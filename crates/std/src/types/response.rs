use {
    crate::Message,
    anyhow::anyhow,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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
            Result::Ok(data) => Self::Ok(data),
            Result::Err(err) => Self::Err(err.to_string()),
        }
    }
}

impl<T> ContractResult<T> {
    pub fn into_result(self) -> anyhow::Result<T> {
        match self {
            ContractResult::Ok(data) => Ok(data),
            ContractResult::Err(err) => Err(anyhow!(err)),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub msgs: Vec<Message>,
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(mut self, msg: Message) -> Self {
        self.msgs.push(msg);
        self
    }
}
