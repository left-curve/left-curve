use {
    crate::{AppError, EventResult, GasTracker, Vm, do_reply, process_msg},
    grug_types::{
        Addr, BlockInfo, Buffer, EventStatus, GenericResult, ReplyOn, Shared, Storage, SubEvent,
        SubEventStatus, SubMessage,
    },
};

/// Maximum number of chained submessages.
///
/// E.g. contract A emits a message to execute contract B, which emits a message
/// to execute C, which emits a message to execute D... so on.
///
/// Without a limit, this can leads to stack overflow which halts the chain.
const MAX_MESSAGE_DEPTH: usize = 30;

macro_rules! try_add_subevent {
    ($events: expr, $submsg_event: expr, $reply: expr) => {
        match $reply {
            EventResult::Ok(evt_reply) => $events.push(EventStatus::Ok(SubEvent {
                event: $submsg_event,
                reply: Some(EventStatus::Ok(evt_reply)),
            })),
            EventResult::Err { event, error } => {
                $events.push(EventStatus::NestedFailed(SubEvent {
                    event: $submsg_event,
                    reply: Some(EventStatus::Failed {
                        event,
                        error: error.to_string(),
                    }),
                }));
                return EventResult::NestedErr {
                    event: $events,
                    error,
                };
            },
            EventResult::NestedErr { event, error } => {
                $events.push(EventStatus::NestedFailed(SubEvent {
                    event: $submsg_event,
                    reply: Some(EventStatus::NestedFailed(event)),
                }));
                return EventResult::NestedErr {
                    event: $events,
                    error,
                };
            },
        };
    };
}

/// Recursively execute submessages emitted in a contract response using a
/// depth-first approach.
///
/// ## Notes
///
/// - The `sender` in this function signature is the contract, i.e. the
///   account that emitted the submessages, not the transaction's sender.
/// - The context for this function requires a boxed storage (`Box<dyn Storage>`)
///   instead of using a generic (`AppCtx<VM, S> where S: Storage`).
///   This is necessary because the function is
pub fn handle_submessages<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    submsgs: Vec<SubMessage>,
) -> EventResult<Vec<EventStatus<SubEvent>>>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let mut events = vec![];

    if msg_depth > MAX_MESSAGE_DEPTH {
        return EventResult::Err {
            event: vec![],
            error: AppError::ExceedMaxMessageDepth,
        };
    }

    for submsg in submsgs {
        let buffer = Shared::new(Buffer::new(storage.clone(), None));
        let result = process_msg(
            vm.clone(),
            Box::new(buffer.clone()),
            gas_tracker.clone(),
            block,
            msg_depth + 1, // important: increase message depth
            sender,
            submsg.msg,
        );

        match (&submsg.reply_on, result.clone().as_result()) {
            // Success - callback requested
            // Flush state changes, log events, give callback.
            (ReplyOn::Success(payload) | ReplyOn::Always(payload), Result::Ok(submsg_event)) => {
                buffer.disassemble().consume();

                let reply = do_reply(
                    vm.clone(),
                    storage.clone(),
                    gas_tracker.clone(),
                    block,
                    msg_depth + 1, // important: increase message depth
                    sender,
                    payload,
                    &GenericResult::Ok(submsg_event.clone()),
                    &submsg.reply_on,
                );

                let submsg_event = SubEventStatus::Ok(submsg_event);

                try_add_subevent!(events, submsg_event, reply);
            },
            // Error - callback requested
            // Discard uncommitted state changes, give callback.
            (
                ReplyOn::Error(payload) | ReplyOn::Always(payload),
                Result::Err((submsg_event, err)),
            ) => {
                let reply = do_reply(
                    vm.clone(),
                    storage.clone(),
                    gas_tracker.clone(),
                    block,
                    msg_depth + 1, // important: increase message depth
                    sender,
                    payload,
                    &GenericResult::Err(err.to_string()),
                    &submsg.reply_on,
                );

                let submsg_event = if reply.is_ok() {
                    SubEventStatus::handled(submsg_event, err)
                } else if let EventResult::Err { .. } = result {
                    SubEventStatus::failed(submsg_event, err)
                } else {
                    SubEventStatus::NestedFailed(submsg_event)
                };

                try_add_subevent!(events, submsg_event, reply);
            },
            // Success - callback not requested
            // Flush state changes, log events, move on to the next submsg.
            (ReplyOn::Error(_), Result::Ok(submsg_event)) => {
                buffer.disassemble().consume();
                events.push(EventStatus::Ok(SubEvent {
                    event: SubEventStatus::Ok(submsg_event),
                    reply: Some(EventStatus::NotReached),
                }));
            },

            (ReplyOn::Never, Result::Ok(submsg_event)) => {
                buffer.disassemble().consume();

                events.push(EventStatus::Ok(SubEvent {
                    event: SubEventStatus::Ok(submsg_event),
                    // Not requested
                    reply: None,
                }));
            },
            // Error - callback not requested
            // Abort by throwing error.
            (ReplyOn::Success(_), Result::Err((_, err))) => {
                events.push(EventStatus::NestedFailed(SubEvent {
                    event: result.into(),
                    reply: None,
                }));

                return EventResult::NestedErr {
                    event: events,
                    error: err,
                };
            },

            (ReplyOn::Never, Result::Err((_, err))) => {
                events.push(EventStatus::NestedFailed(SubEvent {
                    event: result.into(),
                    reply: None,
                }));

                return EventResult::NestedErr {
                    event: events,
                    error: err,
                };
            },
        };
    }

    EventResult::Ok(events)
}
