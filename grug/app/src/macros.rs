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

                return EventResult::NestedErr { event: $evt, error };
            },
            EventResult::NestedErr { event, error } => {
                $evt.$field = grug_types::EventStatus::NestedFailed(event);

                return EventResult::NestedErr { event: $evt, error };
            },
        }
    };
}

#[macro_export]
macro_rules! catch_and_push_event {
    ($result:expr, $evt:expr, $field:ident) => {
        match $result {
            EventResult::Ok(i) => {
                $evt.$field.push(grug_types::EventStatus::Ok(i));
            },
            EventResult::Err { event, error } => {
                $evt.$field.push(grug_types::EventStatus::Failed {
                    event,
                    error: error.to_string(),
                });

                return EventResult::NestedErr { event: $evt, error };
            },
            EventResult::NestedErr { event, error } => {
                $evt.$field.push(grug_types::EventStatus::NestedFailed(event));

                return EventResult::NestedErr { event: $evt, error };
            },
        }
    };
}

#[macro_export]
macro_rules! catch_and_insert_event {
    ($result:expr, $evt:expr, $field:ident, key:$key:expr) => {
        match $result {
            EventResult::Ok(i) => {
                $evt.$field.insert($key, grug_types::EventStatus::Ok(i));
            },
            EventResult::Err { event, error } => {
                $evt.$field.insert($key, grug_types::EventStatus::Failed {
                    event,
                    error: error.to_string(),
                });

                return EventResult::NestedErr { event: $evt, error };
            },
            EventResult::NestedErr { event, error } => {
                $evt.$field.insert($key, grug_types::EventStatus::NestedFailed(event));

                return EventResult::NestedErr { event: $evt, error };
            },
        }
    };
}
