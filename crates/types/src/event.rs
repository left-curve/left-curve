use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub r#type: String,
    pub attributes: Vec<Attribute>,
}

impl Event {
    pub fn new(ty: impl ToString) -> Self {
        Self {
            r#type: ty.to_string(),
            attributes: vec![],
        }
    }

    pub fn add_attribute(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.attributes.push(Attribute::new(key, value));
        self
    }

    pub fn add_attributes(mut self, attrs: impl IntoIterator<Item = Attribute>) -> Self {
        self.attributes.extend(attrs);
        self
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

impl Attribute {
    pub fn new(key: impl ToString, value: impl ToString) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}
