use {
    crate::AppError,
    grug_types::{CommitmentStatus, Event, HandleEventStatus},
};

#[derive(Debug, Clone)]
pub enum EventResult<T> {
    Ok(T),
    Err { event: T, error: AppError },
    SubErr { event: T, error: AppError },
}

impl<T> EventResult<T> {
    pub fn err(event: T, error: AppError) -> Self {
        EventResult::Err { event, error }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, EventResult::Ok(_))
    }

    pub fn map<C, R>(self, callback: C) -> EventResult<R>
    where
        C: Fn(T) -> R,
    {
        match self {
            EventResult::Ok(event) => EventResult::Ok(callback(event)),
            EventResult::Err { event, error } => EventResult::Err {
                event: callback(event),
                error,
            },
            EventResult::SubErr { event, error } => EventResult::SubErr {
                event: callback(event),
                error,
            },
        }
    }

    pub fn map_merge<R, C>(self, merge: R, callback: C) -> EventResult<R>
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

    pub fn as_result(self) -> Result<T, (T, AppError)> {
        match self {
            EventResult::Ok(val) => Ok(val),
            EventResult::Err { event, error } | EventResult::SubErr { event, error } => {
                Err((event, error))
            },
        }
    }

    pub fn as_committment(self) -> CommitmentStatus<T> {
        match self {
            EventResult::Ok(event) => CommitmentStatus::Committed(event),
            EventResult::Err { event, error } | EventResult::SubErr { event, error } => {
                CommitmentStatus::Failed {
                    event,
                    error: error.to_string(),
                }
            },
        }
    }

    #[cfg(feature = "tracing")]
    pub fn debug<O>(&self, ok_closure: O, error_msg: &str)
    where
        O: Fn(&T),
    {
        match self {
            EventResult::Ok(val) => {
                ok_closure(val);
            },
            EventResult::Err { error, .. } => {
                tracing::warn!(err = error.to_string(), error_msg);
            },
            EventResult::SubErr { error, .. } => {
                tracing::warn!(err = error.to_string(), "Sub error encountered");
            },
        }
    }
}

impl From<EventResult<Event>> for HandleEventStatus {
    fn from(value: EventResult<Event>) -> Self {
        match value {
            EventResult::Ok(event) => HandleEventStatus::Ok(event),
            EventResult::Err { event, error } => HandleEventStatus::failed(event, error),
            EventResult::SubErr { event, .. } => HandleEventStatus::NestedFailed(event),
        }
    }
}
