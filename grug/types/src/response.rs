use {
    crate::{Addr, Json, JsonSerExt, Message, StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default, Debug, Clone, PartialEq, Eq,
)]
pub struct Response {
    pub submsgs: Vec<SubMessage>,
    pub subevents: Vec<ContractEvent>,
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message<M>(mut self, msg: M) -> Self
    where
        M: Into<Message>,
    {
        self.submsgs.push(SubMessage::reply_never(msg));
        self
    }

    pub fn may_add_message<M>(mut self, maybe_msg: Option<M>) -> Self
    where
        M: Into<Message>,
    {
        if let Some(msg) = maybe_msg {
            self.submsgs.push(SubMessage::reply_never(msg));
        }
        self
    }

    pub fn add_messages<M, I>(mut self, msgs: I) -> Self
    where
        M: Into<Message>,
        I: IntoIterator<Item = M>,
    {
        self.submsgs
            .extend(msgs.into_iter().map(SubMessage::reply_never));
        self
    }

    pub fn add_submessage(mut self, submsg: SubMessage) -> Self {
        self.submsgs.push(submsg);
        self
    }

    pub fn may_add_submessage(mut self, maybe_submsg: Option<SubMessage>) -> Self {
        if let Some(submsg) = maybe_submsg {
            self.submsgs.push(submsg);
        }
        self
    }

    pub fn add_submessages<M>(mut self, submsgs: M) -> Self
    where
        M: IntoIterator<Item = SubMessage>,
    {
        self.submsgs.extend(submsgs);
        self
    }

    pub fn add_event<E>(mut self, event: E) -> StdResult<Self>
    where
        E: TryInto<ContractEvent>,
        StdError: From<E::Error>,
    {
        self.subevents.push(event.try_into()?);
        Ok(self)
    }

    pub fn may_add_event<E>(mut self, maybe_event: Option<E>) -> StdResult<Self>
    where
        E: TryInto<ContractEvent>,
        StdError: From<E::Error>,
    {
        if let Some(event) = maybe_event {
            self.subevents.push(event.try_into()?);
        }
        Ok(self)
    }

    pub fn add_events<I>(mut self, events: I) -> Self
    where
        I: IntoIterator<Item = ContractEvent>,
    {
        self.subevents.extend(events);
        self
    }
}

/// A special response emitted by the account contract at the end of the
/// `authenticate` method call. In addition to the usual [`Response`](crate::Response),
/// this also includes a boolean specifying whether the account requests a
/// backrun call.
#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default, Debug, Clone, PartialEq, Eq,
)]
pub struct AuthResponse {
    pub response: Response,
    pub request_backrun: bool,
}

impl AuthResponse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_backrun(mut self, request_backrun: bool) -> Self {
        self.request_backrun = request_backrun;
        self
    }

    pub fn add_message(mut self, msg: Message) -> Self {
        self.response = self.response.add_message(msg);
        self
    }

    pub fn may_add_message(mut self, maybe_msg: Option<Message>) -> Self {
        self.response = self.response.may_add_message(maybe_msg);
        self
    }

    pub fn add_messages<M>(mut self, msgs: M) -> Self
    where
        M: IntoIterator<Item = Message>,
    {
        self.response = self.response.add_messages(msgs);
        self
    }

    pub fn add_submessage(mut self, submsg: SubMessage) -> Self {
        self.response = self.response.add_submessage(submsg);
        self
    }

    pub fn may_add_submessage(mut self, maybe_submsg: Option<SubMessage>) -> Self {
        self.response = self.response.may_add_submessage(maybe_submsg);
        self
    }

    pub fn add_submessages<M>(mut self, submsgs: M) -> Self
    where
        M: IntoIterator<Item = SubMessage>,
    {
        self.response = self.response.add_submessages(submsgs);
        self
    }

    pub fn add_event<E>(mut self, event: E) -> StdResult<Self>
    where
        E: TryInto<ContractEvent>,
        StdError: From<E::Error>,
    {
        self.response = self.response.add_event(event)?;
        Ok(self)
    }

    pub fn may_add_event<E>(mut self, maybe_event: Option<E>) -> StdResult<Self>
    where
        E: TryInto<ContractEvent>,
        StdError: From<E::Error>,
    {
        self.response = self.response.may_add_event(maybe_event)?;
        Ok(self)
    }

    pub fn add_events<I>(mut self, events: I) -> Self
    where
        I: IntoIterator<Item = ContractEvent>,
    {
        self.response = self.response.add_events(events);
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
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplyOn {
    Success(Json),
    Error(Json),
    Always(Json),
    Never,
}

impl ReplyOn {
    pub fn success<T>(callback: &T) -> StdResult<Self>
    where
        T: Serialize,
    {
        callback.to_json_value().map(Self::Success)
    }

    pub fn error<T>(callback: &T) -> StdResult<Self>
    where
        T: Serialize,
    {
        callback.to_json_value().map(Self::Error)
    }

    pub fn always<T>(callback: &T) -> StdResult<Self>
    where
        T: Serialize,
    {
        callback.to_json_value().map(Self::Always)
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct SubMessage {
    pub msg: Message,
    pub reply_on: ReplyOn,
}

impl SubMessage {
    pub fn reply_never<M>(msg: M) -> Self
    where
        M: Into<Message>,
    {
        Self {
            msg: msg.into(),
            reply_on: ReplyOn::Never,
        }
    }

    pub fn reply_always<M, P>(msg: M, payload: &P) -> StdResult<Self>
    where
        M: Into<Message>,
        P: Serialize,
    {
        Ok(Self {
            msg: msg.into(),
            reply_on: ReplyOn::Always(payload.to_json_value()?),
        })
    }

    pub fn reply_on_success<M, P>(msg: M, payload: &P) -> StdResult<Self>
    where
        M: Into<Message>,
        P: Serialize,
    {
        Ok(Self {
            msg: msg.into(),
            reply_on: ReplyOn::Success(payload.to_json_value()?),
        })
    }

    pub fn reply_on_error<M, P>(msg: M, payload: &P) -> StdResult<Self>
    where
        M: Into<Message>,
        P: Serialize,
    {
        Ok(Self {
            msg: msg.into(),
            reply_on: ReplyOn::Error(payload.to_json_value()?),
        })
    }
}

/// A [ContractEvent] where the contract address is added by the host.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct CheckedContractEvent {
    pub contract: Addr,
    #[serde(rename = "type")]
    pub ty: String,
    pub data: Json,
}

impl CheckedContractEvent {
    pub fn new<T, U>(contract: Addr, ty: T, data: U) -> StdResult<Self>
    where
        T: Into<String>,
        U: Serialize,
    {
        Ok(Self {
            contract,
            ty: ty.into(),
            data: data.to_json_value()?,
        })
    }
}

/// An event emitted by a contract, containing an arbitrary string identifying
/// its type and an arbitrary JSON data.
///
/// In grug-app, this is converted to an [`Event::Guest`](crate::Event).
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ContractEvent {
    #[serde(rename = "type")]
    pub ty: String,
    pub data: Json,
}

impl ContractEvent {
    pub fn new<T, U>(ty: T, data: U) -> StdResult<Self>
    where
        T: Into<String>,
        U: Serialize,
    {
        Ok(Self {
            ty: ty.into(),
            data: data.to_json_value()?,
        })
    }
}
