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
    /// Create a new `EventResult<R>`.
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

#[macro_export]
macro_rules! catch_event {
    ($block:block, $evt:expr) => {
        match (|| $block)() {
            Ok(val) => val,
            Err(err) => {
                return $crate::EventResult::Err {
                    event: $evt,
                    error: err,
                };
            },
        }
    };
}

#[macro_export]
macro_rules! catch_and_update_event {
    ($result:expr, $evt:expr => $field:ident) => {
        match $result {
            EventResult::Ok(i) => {
                $evt.$field = grug_types::EventStatus::Ok(i);
            },
            EventResult::Err { event, error } => {
                $evt.$field = grug_types::EventStatus::Failed {
                    event,
                    error: error.to_string(),
                };

                return EventResult::SubErr { event: $evt, error };
            },
            EventResult::SubErr { event, error } => {
                $evt.$field = grug_types::EventStatus::NestedFailed(event);

                return EventResult::SubErr { event: $evt, error };
            },
        }
    };
}

#[macro_export]
macro_rules! catch_and_append_event {
    ($result:expr, $evt:expr) => {
        match $result {
            EventResult::Ok(i) => {
                $evt.msgs.push(grug_types::EventStatus::Ok(i));
            },
            EventResult::Err { event, error } => {
                $evt.msgs.push(grug_types::EventStatus::Failed {
                    event,
                    error: error.to_string(),
                });

                return EventResult::SubErr { event: $evt, error };
            },
            EventResult::SubErr { event, error } => {
                $evt.msgs.push(grug_types::EventStatus::NestedFailed(event));

                return EventResult::SubErr { event: $evt, error };
            },
        }
    };
}
