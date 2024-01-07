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
    pub msgs:       Vec<Message>,
    pub attributes: Vec<Attribute>,
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(mut self, msg: Message) -> Self {
        self.msgs.push(msg);
        self
    }

    pub fn add_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push(Attribute::new(key, value));
        self
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    key:   String,
    value: String,
}

impl Attribute {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key:   key.into(),
            value: value.into(),
        }
    }
}
