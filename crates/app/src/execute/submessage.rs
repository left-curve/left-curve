use {
    super::new_reply_event,
    crate::{process_msg, AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONTRACT_NAMESPACE},
    cw_db::{CacheStore, PrefixStore, SharedStore},
    cw_std::{Addr, Binary, BlockInfo, Context, Event, GenericResult, ReplyOn, Storage, SubMessage},
    cw_vm::Instance,
    tracing::{info, warn},
};

/// Recursively execute submessages emitted in a contract response using a
/// depth-first approach.
///
/// Note: The `sender` in this function signature is the contract, i.e. the
/// account that emitted the submessages, not the transaction's sender.
pub fn handle_submessages<S: Storage + Clone + 'static>(
    store:   S,
    block:   &BlockInfo,
    sender:  &Addr,
    submsgs: Vec<SubMessage>,
) -> AppResult<Vec<Event>> {
    let mut events = vec![];
    for submsg in submsgs {
        let cached = SharedStore::new(CacheStore::new(store.clone(), None));
        match (submsg.reply_on, process_msg(cached.share(), block, sender, submsg.msg)) {
            // success - callback requested
            // flush state changes, log events, give callback
            (ReplyOn::Success(payload) | ReplyOn::Always(payload), Result::Ok(submsg_events)) => {
                cached.disassemble().consume();
                events.extend(submsg_events.clone());
                events.extend(reply(
                    store.clone(),
                    block,
                    sender,
                    payload,
                    GenericResult::Ok(submsg_events),
                )?);
            },
            // error - callback requested
            // discard uncommitted state changes, give callback
            (ReplyOn::Error(payload) | ReplyOn::Always(payload), Result::Err(err)) => {
                events.extend(reply(
                    store.clone(),
                    block,
                    sender,
                    payload,
                    GenericResult::Err(err.to_string()),
                )?);
            },
            // success - callback not requested
            // flush state changes, log events, move on to the next submsg
            (ReplyOn::Error(_) | ReplyOn::Never, Result::Ok(submsg_events)) => {
                cached.disassemble().consume();
                events.extend(submsg_events);
            },
            // error - callback not requested
            // abort by throwing error
            (ReplyOn::Success(_) | ReplyOn::Never, Result::Err(err)) => {
                return Err(err);
            },
        };
    }
    Ok(events)
}

fn reply<S: Storage + Clone + 'static>(
    store:         S,
    block:         &BlockInfo,
    contract:      &Addr,
    payload:       Binary,
    submsg_result: GenericResult<Vec<Event>>,
) -> AppResult<Vec<Event>> {
    match _reply(store, block, contract, payload, submsg_result) {
        Ok(events) => {
            info!(contract = contract.to_string(), "Performed callback");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to perform callback");
            Err(err)
        },
    }
}

fn _reply<S: Storage + Clone + 'static>(
    store:         S,
    block:         &BlockInfo,
    contract:      &Addr,
    payload:       Binary,
    submsg_result: GenericResult<Vec<Event>>,
) -> AppResult<Vec<Event>> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &contract]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call reply
    let ctx = Context {
        chain_id,
        block:         block.clone(),
        contract:      contract.clone(),
        sender:        None,
        funds:         None,
        simulate:      None,
        submsg_result: Some(submsg_result),
    };
    let resp = instance.call_reply(&ctx, payload)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_reply_event(contract, resp.attributes)];
    events.extend(handle_submessages(store, block, contract, resp.submsgs)?);

    Ok(events)
}
