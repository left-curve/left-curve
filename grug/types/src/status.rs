use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_backtrace::BacktracedError,
    serde::{Deserialize, Serialize},
};

/// Describes whether a set of states changes have been committed to the chain
/// state.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentStatus<T> {
    /// The state changes have been committed.
    Committed(T),
    /// The state changes have been discarded because its execution failed.
    Failed {
        event: T,
        error: BacktracedError<String>,
    },
    /// The state changes have been discarded, despite its execution was
    /// successful, but some other parts of the transaction execution flow
    /// failed; specifically, the `finalize_fee` call on taxman.
    Reverted {
        event: T,
        revert_by: BacktracedError<String>,
    },
    /// The execution was not reached because earlier parts of the transaction
    /// execution flow failed.
    NotReached,
}

impl<T> CommitmentStatus<T> {
    pub fn maybe_error(&self) -> Option<&BacktracedError<String>> {
        match self {
            Self::Failed { error, .. }
            | Self::Reverted {
                revert_by: error, ..
            } => Some(error),
            _ => None,
        }
    }

    pub fn as_result(&self) -> Result<&T, (&T, &BacktracedError<String>)> {
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
    Failed {
        event: T,
        error: BacktracedError<String>,
    },
    /// Not reached because a previous event failed.
    NotReached,
}
