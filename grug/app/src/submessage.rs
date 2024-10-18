use {
    crate::{do_reply, process_msg, AppCtx, AppError, AppResult, Buffer, Shared, Vm},
    grug_types::{Addr, Event, GenericResult, ReplyOn, SubMessage},
};

/// Maximum number of chained submessages.
///
/// E.g. contract A emits a message to execute contract B, which emits a message
/// to execute C, which emits a message to execute D... so on.
///
/// Without a limit, this can leads to stack overflow which halts the chain.
const MAX_MESSAGE_DEPTH: usize = 30;

/// Recursively execute submessages emitted in a contract response using a
/// depth-first approach.
///
/// Note: The `sender` in this function signature is the contract, i.e. the
/// account that emitted the submessages, not the transaction's sender.
pub fn handle_submessages<VM>(
    ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    submsgs: Vec<SubMessage>,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut events = vec![];

    if msg_depth > MAX_MESSAGE_DEPTH {
        return Err(AppError::ExceedMaxMessageDepth);
    }

    for submsg in submsgs {
        let buffer = Shared::new(Buffer::new(ctx.storage.clone(), None));
        let result = process_msg(
            ctx.clone_with_storage(Box::new(buffer.clone())),
            msg_depth + 1, // important: increase message depth
            sender,
            submsg.msg,
        );

        match (submsg.reply_on, result) {
            // Success - callback requested
            // Flush state changes, log events, give callback.
            (ReplyOn::Success(payload) | ReplyOn::Always(payload), Result::Ok(submsg_events)) => {
                buffer.disassemble().consume();
                events.extend(submsg_events.clone());
                events.extend(do_reply(
                    ctx.clone(),
                    msg_depth + 1, // important: increase message depth
                    sender,
                    &payload,
                    &GenericResult::Ok(submsg_events),
                )?);
            },
            // Error - callback requested
            // Discard uncommitted state changes, give callback.
            (ReplyOn::Error(payload) | ReplyOn::Always(payload), Result::Err(err)) => {
                events.extend(do_reply(
                    ctx.clone(),
                    msg_depth + 1, // important: increase message depth
                    sender,
                    &payload,
                    &GenericResult::Err(err.to_string()),
                )?);
            },
            // Success - callback not requested
            // Flush state changes, log events, move on to the next submsg.
            (ReplyOn::Error(_) | ReplyOn::Never, Result::Ok(submsg_events)) => {
                buffer.disassemble().consume();
                events.extend(submsg_events);
            },
            // Error - callback not requested
            // Abort by throwing error.
            (ReplyOn::Success(_) | ReplyOn::Never, Result::Err(err)) => {
                return Err(err);
            },
        };
    }

    Ok(events)
}
