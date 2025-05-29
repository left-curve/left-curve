use {
    crate::AppError,
    grug_types::{CommitmentStatus, Event, EventStatus, SubEventStatus},
};

#[derive(Debug, Clone)]
pub enum EventResult<T> {
    Ok(T),
    Err { event: T, error: AppError },
    NestedErr { event: T, error: AppError },
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
            EventResult::NestedErr { event, error } => EventResult::NestedErr {
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
            EventResult::Err { event, error } | EventResult::NestedErr { event, error } => {
                EventResult::NestedErr {
                    event: callback(event, merge),
                    error,
                }
            },
        }
    }

    pub fn as_result(self) -> Result<T, (T, AppError)> {
        match self {
            EventResult::Ok(val) => Ok(val),
            EventResult::Err { event, error } | EventResult::NestedErr { event, error } => {
                Err((event, error))
            },
        }
    }

    pub fn into_commitment_status(self) -> CommitmentStatus<EventStatus<T>> {
        match &self {
            EventResult::Ok(..) => CommitmentStatus::Committed(self.into()),
            EventResult::Err { error, .. } | EventResult::NestedErr { error, .. } => {
                CommitmentStatus::Failed {
                    error: error.to_string(),
                    event: self.into(),
                }
            },
        }
    }

    pub fn into_commitment(self) -> CommitmentStatus<T> {
        match self {
            EventResult::Ok(event) => CommitmentStatus::Committed(event),
            EventResult::Err { event, error } | EventResult::NestedErr { event, error } => {
                CommitmentStatus::Failed {
                    error: error.to_string(),
                    event,
                }
            },
        }
    }

    #[cfg(feature = "tracing")]
    pub fn debug<O>(&self, ok_closure: O, error_msg: &str, error_level: tracing::Level)
    where
        O: Fn(&T),
    {
        use crate::dyn_event;

        match self {
            EventResult::Ok(val) => {
                ok_closure(val);
            },
            EventResult::Err { error, .. } | EventResult::NestedErr { error, .. } => {
                dyn_event!(error_level, err = error.to_string(), error_msg);
            },
        }
    }
}

impl From<EventResult<Event>> for SubEventStatus {
    fn from(value: EventResult<Event>) -> Self {
        match value {
            EventResult::Ok(event) => SubEventStatus::Ok(event),
            EventResult::Err { event, error } => SubEventStatus::failed(event, error),
            EventResult::NestedErr { event, .. } => SubEventStatus::NestedFailed(event),
        }
    }
}

impl<T> From<EventResult<T>> for EventStatus<T> {
    fn from(value: EventResult<T>) -> Self {
        match value {
            EventResult::Ok(event) => EventStatus::Ok(event),
            EventResult::Err { event, error } => EventStatus::Failed {
                event,
                error: error.to_string(),
            },
            EventResult::NestedErr { event, .. } => EventStatus::NestedFailed(event),
        }
    }
}
