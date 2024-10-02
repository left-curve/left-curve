use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub r#type: String,
    pub attributes: Vec<Attribute>,
}

impl Event {
    pub fn new<T>(ty: T) -> Self
    where
        T: ToString,
    {
        Self {
            r#type: ty.to_string(),
            attributes: vec![],
        }
    }

    pub fn add_attribute<K, V>(mut self, key: K, value: V) -> Self
    where
        K: ToString,
        V: ToString,
    {
        self.attributes.push(Attribute::new(key, value));
        self
    }

    pub fn add_attributes<A>(mut self, attrs: A) -> Self
    where
        A: IntoIterator<Item = Attribute>,
    {
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
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        K: ToString,
        V: ToString,
    {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}
