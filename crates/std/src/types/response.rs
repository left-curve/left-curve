use {
    crate::{Attribute, Message},
    serde::{Deserialize, Serialize},
};

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

    pub fn add_attribute(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.attributes.push(Attribute::new(key, value));
        self
    }
}
