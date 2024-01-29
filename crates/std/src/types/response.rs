use {
    crate::{to_json, Attribute, Binary, Message, StdResult},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub submsgs: Vec<SubMessage>,
    pub attributes: Vec<Attribute>,
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(mut self, msg: Message) -> Self {
        self.submsgs.push(SubMessage::reply_never(msg));
        self
    }

    pub fn add_messages(mut self, msgs: impl IntoIterator<Item = Message>) -> Self {
        self.submsgs.extend(msgs.into_iter().map(SubMessage::reply_never));
        self
    }

    pub fn add_submessage(mut self, submsg: SubMessage) -> Self {
        self.submsgs.push(submsg);
        self
    }

    pub fn add_submessages(mut self, submsgs: impl IntoIterator<Item = SubMessage>) -> Self {
        self.submsgs.extend(submsgs);
        self
    }

    pub fn add_attribute(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.attributes.push(Attribute::new(key, value));
        self
    }
}

/// Indicates that after a submessage has been executed, whether the host should
/// give the contract a callack.
///
/// The host's behavior is summariazed in the table below:
///
/// result | Success   | Error    | Always   | Never   |
/// ------ | --------- | -------- | -------- | ------- |
/// Ok     | callback  | nothing  | callback | nothing |
/// Err    | abort     | callback | callback | abort   |
///
/// In case a callback is to be performed, the host passes a piece of binary
/// payload data to the contract.
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
