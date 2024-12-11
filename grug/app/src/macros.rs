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
