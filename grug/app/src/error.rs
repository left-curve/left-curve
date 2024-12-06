use {
    grug_types::{Addr, Hash256, StdError},
    thiserror::Error,
};

#[derive(Debug, Clone, Error)]
pub enum AppError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("VM error: {0}")]
    Vm(String),

    #[error("DB error: {0}")]
    Db(String),

    #[error("proposal preparer error: {0}")]
    PrepareProposal(String),

    #[error("indexer error: {0}")]
    Indexer(String),

    #[error("contract returned error! address: {address}, method: {name}, msg: {msg}")]
    Guest {
        address: Addr,
        name: &'static str,
        msg: String,
    },

    #[error("merkle proof is not supported for `/app` query; use `/store` instead")]
    ProofNotSupported,

    #[error("simulating a transaction at past block height is not supported")]
    PastHeightNotSupported,

    #[error("sender does not have permission to perform this action")]
    Unauthorized,

    #[error("incorrect block height! expecting: {expect}, actual: {actual}")]
    IncorrectBlockHeight { expect: u64, actual: u64 },

    #[error("sender is not the owner! sender: {sender}, owner: {owner}")]
    NotOwner { sender: Addr, owner: Addr },

    #[error("admin account is not set")]
    AdminNotSet,

    #[error("sender is not the admin! sender: {sender}, admin: {admin}")]
    NotAdmin { sender: Addr, admin: Addr },

    #[error("code with hash `{code_hash}` already exists")]
    CodeExists { code_hash: Hash256 },

    #[error("account with address `{address}` already exists")]
    AccountExists { address: Addr },

    #[error("max message depth exceeded")]
    ExceedMaxMessageDepth,
}

pub type AppResult<T> = core::result::Result<T, AppError>;

pub enum EventResult<T> {
    Ok(T),
    Err { event: T, error: AppError },
    SubErr { event: T, error: AppError },
}

impl<T> EventResult<T> {
    pub fn upcast<R, C>(self, merge: R, callback: C) -> EventResult<R>
    where
        C: Fn(T, R) -> R,
    {
        match self {
            EventResult::Ok(event) => EventResult::Ok(callback(event, merge)),
            EventResult::Err { event, error } | EventResult::SubErr { event, error } => {
                EventResult::SubErr {
                    event: callback(event, merge),
                    error,
                }
            },
        }
    }

    pub fn err(event: T, error: AppError) -> Self {
        EventResult::Err { event, error }
    }

    pub fn debug<O>(&self, ok_closure: O, error_msg: &str)
    where
        O: Fn(&T),
    {
        match self {
            EventResult::Ok(val) => ok_closure(val),
            EventResult::Err { error, .. } => {
                tracing::warn!(err = error.to_string(), error_msg);
            },
            EventResult::SubErr { .. } => {
                tracing::warn!("Sub Error encountered");
            },
        }
    }

    pub fn as_result(self) -> Result<T, (T, AppError)> {
        match self {
            EventResult::Ok(val) => Ok(val),
            EventResult::Err { event, error } | EventResult::SubErr { event, error } => {
                Err((event, error))
            },
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, EventResult::Ok(_))
    }
}

#[macro_export]
macro_rules! catch_event {
    ($evt: expr, $block: block) => {{
        match (|| $block)() {
            Ok(val) => val,
            Err(err) => return $crate::EventResult::Err {
                event: $evt,
                error: err,
            },
        }
    }};
}

#[macro_export]
macro_rules! update_event_field {
    ($result: expr, $evt: expr => $field: ident) => {
        match $result {
            EventResult::Ok(i) => $evt.$field = EventStatus::Ok(i),
            EventResult::Err { event, error } => {
                $evt.$field = EventStatus::Failed {
                    event,
                    error: error.to_string(),
                };
                return EventResult::SubErr { event: $evt, error };
            },
            EventResult::SubErr { event, error } => {
                $evt.$field = EventStatus::Ok(event);
                return EventResult::SubErr { event: $evt, error };
            },
        };
    };
}
