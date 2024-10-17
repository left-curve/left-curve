use {
    crate::{do_reply, process_msg, AppError, AppResult, Buffer, GasTracker, Shared, Vm},
    grug_types::{Addr, BlockInfo, Config, Event, GenericResult, ReplyOn, Storage, SubMessage},
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
    vm: VM,
    cfg: &Config,
    // This function takes a boxed store instead of using a generic like others.
    //
    // This is because this function is recursive: every layer of recursion, it
    // wraps the store with `Shared<Buffer<S>>`.
    //
    // Although the recursion is guaranteed to be bounded at run time (thanks to
    // gas limit), the compiler can't understand this. The compiler thinks the
    // wrapping can possibly go on infinitely. It would throw this error:
    //
    // > error: reached the recursion limit while instantiating
    // > `process_msg::<Shared<Buffer<Shared<Buffer<Shared<...>>>>>>`
    //
    // To prevent this, we use `Box<dyn Storage>` instead, which is an opaque
    // type, so that the compiler does not think about how many layers of
    // wrapping there are.
    //
    // Another complexity involved here is that we need the store to be clonable.
    // However we can't write `Box<dyn Storage + Clone>` because `Clone` is not
    // an object-safe trait:
    // https://doc.rust-lang.org/reference/items/traits.html#object-safety
    //
    // Instead, we use the `dyn_clone::DynClone` trait:
    // https://docs.rs/dyn-clone/1.0.16/dyn_clone/
    storage: Box<dyn Storage>,
    block: BlockInfo,
    gas_tracker: GasTracker,
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
        let buffer = Shared::new(Buffer::new(storage.clone(), None));
        let result = process_msg(
            vm.clone(),
            cfg,
            Box::new(buffer.clone()),
            gas_tracker.clone(),
            msg_depth + 1, // important: increase message depth
            block,
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
                    vm.clone(),
                    cfg,
                    storage.clone(),
                    gas_tracker.clone(),
                    msg_depth + 1, // important: increase message depth
                    block,
                    sender,
                    &payload,
                    &GenericResult::Ok(submsg_events),
                )?);
            },
            // Error - callback requested
            // Discard uncommitted state changes, give callback.
            (ReplyOn::Error(payload) | ReplyOn::Always(payload), Result::Err(err)) => {
                events.extend(do_reply(
                    vm.clone(),
                    cfg,
                    storage.clone(),
                    gas_tracker.clone(),
                    msg_depth + 1, // important: increase message depth
                    block,
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
