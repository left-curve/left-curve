use {
    crate::Event,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentStatus<T> {
    Committed(T),
    Failed { event: T, error: String },
    Reverted { event: T, revert_by: String },
    NotReached,
}

impl<T> CommitmentStatus<T> {
    pub fn maybe_error(&self) -> Option<&str> {
        match self {
            Self::Failed { error, .. }
            | Self::Reverted {
                revert_by: error, ..
            } => Some(error),
            _ => None,
        }
    }

    pub fn as_result(&self) -> Result<&T, (&T, &str)> {
        match self {
            Self::Committed(event) => Ok(event),
            Self::Failed { event, error }
            | Self::Reverted {
                event,
                revert_by: error,
            } => Err((event, error)),
            Self::NotReached => panic!("not reached"),
        }
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus<T> {
    /// The event succeeded.
    Ok(T),
    /// A nested event failed.
    NestedFailed(T),
    /// The event failed.
    Failed { event: T, error: String },
    /// Not reached because a previous event failed.
    NotReached,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandleEventStatus {
    /// The event succeeded.
    /// State changes are committed.
    Ok(Event),
    /// A nested event failed.
    NestedFailed(Event),
    /// The event failed.
    /// State changes are reverted.
    Failed { event: Event, error: String },
    /// The event failed but the error was handled.
    /// State changes are reverted but the tx continues.
    Handled { event: Event, error: String },
}

impl HandleEventStatus {
    pub fn failed<E>(event: Event, error: E) -> Self
    where
        E: ToString,
    {
        Self::Failed {
            event,
            error: error.to_string(),
        }
    }

    pub fn handled<E>(event: Event, error: E) -> Self
    where
        E: ToString,
    {
        Self::Handled {
            event,
            error: error.to_string(),
        }
    }
}

impl From<EventStatus<Event>> for HandleEventStatus {
    fn from(value: EventStatus<Event>) -> Self {
        match value {
            EventStatus::Ok(e) => HandleEventStatus::Ok(e),
            EventStatus::NestedFailed(e) => HandleEventStatus::NestedFailed(e),
            EventStatus::Failed { event, error } => HandleEventStatus::Failed { event, error },
            EventStatus::NotReached => unreachable!(),
        }
    }
}
