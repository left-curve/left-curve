use {
    crate::{to_json, Attribute, Binary, Message, StdResult},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub messages: Vec<SubMessage>,
    pub attributes: Vec<Attribute>,
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(mut self, msg: Message) -> Self {
        self.messages.push(SubMessage::reply_never(msg));
        self
    }

    pub fn add_messages(mut self, msgs: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(msgs.into_iter().map(SubMessage::reply_never));
        self
    }

    pub fn add_submsg(mut self, submsg: SubMessage) -> Self {
        self.messages.push(submsg);
        self
    }

    pub fn add_submsgs(mut self, submsgs: impl IntoIterator<Item = SubMessage>) -> Self {
        self.messages.extend(submsgs);
        self
    }

    pub fn add_attribute(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.attributes.push(Attribute::new(key, value));
        self
    }
}

/// result | Success   | Error    | Always   | Never   |
/// ------ | --------- | -------- | -------- | ------- |
/// Ok     | callback  | nothing  | callback | nothing |
/// Err    | abort     | callback | callback | abort   |
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ReplyOn {
    Success(Binary),
    Error(Binary),
    Always(Binary),
    Never,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SubMessage {
    pub msg: Message,
    pub reply_on: ReplyOn,
}

impl SubMessage {
    pub fn reply_never(msg: Message) -> Self {
        Self {
            msg,
            reply_on: ReplyOn::Never,
        }
    }

    pub fn reply_always<P: Serialize>(msg: Message, payload: &P) -> StdResult<Self> {
        Ok(Self {
            msg,
            reply_on: ReplyOn::Always(to_json(payload)?),
        })
    }

    pub fn reply_on_success<P: Serialize>(msg: Message, payload: &P) -> StdResult<Self> {
        Ok(Self {
            msg,
            reply_on: ReplyOn::Success(to_json(payload)?),
        })
    }

    pub fn reply_on_error<P: Serialize>(msg: Message, payload: &P) -> StdResult<Self> {
        Ok(Self {
            msg,
            reply_on: ReplyOn::Error(to_json(payload)?),
        })
    }
}
