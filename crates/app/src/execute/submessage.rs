use {
    crate::{process_msg, AppResult},
    cw_std::{Addr, BlockInfo, Event, Message, Storage},
};

/// Recursively execute submessages emitted in a contract response using a
/// depth-first approach.
///
/// Note: The `sender` in this function signature is the contract, i.e. the
/// account that emitted the submessages, not the transaction's sender.
pub fn handle_submessages<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    sender:   &Addr,
    messages: Vec<Message>,
) -> AppResult<Vec<Event>> {
    let mut events = vec![];
    for msg in messages {
        events.extend(process_msg(store.clone(), block, sender, msg)?);
    }
    Ok(events)
}
