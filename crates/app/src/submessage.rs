use {
    super::new_reply_event,
    crate::{process_msg, AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONTRACT_NAMESPACE},
    cw_db::{CacheStore, PrefixStore, SharedStore},
    cw_std::{
        Addr, BlockInfo, Context, Event, GenericResult, Json, ReplyOn, Storage, SubMessage,
        SubMsgResult,
    },
    cw_vm::Instance,
    tracing::{info, warn},
};

/// Recursively execute submessages emitted in a contract response using a
/// depth-first approach.
///
/// Note: The `sender` in this function signature is the contract, i.e. the
/// account that emitted the submessages, not the transaction's sender.
pub fn handle_submessages(
    // This function takes a boxed store instead of using a generic like others.
    //
    // This is because this function is recursive: every layer of recursion, it
    // wraps the store with `SharedStore<CacheStore<S>>`.
    //
    // Although the recursion is guaranteed to be bounded at run time (thanks to
    // gas limit), the compiler can't understand this. The compiler thinks the
    // wrapping can possibly go on infinitely. It would throw this error:
    //
    // > error: reached the recursion limit while instantiating
    // > `process_msg::<SharedStore<CacheStore<SharedStore<CacheStore<SharedStore<...>>>>>>`
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
    //
    // TODO: these wrapping and boxing for sure has impact on performance.
    // This is probably fine for now, but we should think about optimizing this.
    store:   Box<dyn Storage>,
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
                events.extend(do_reply(
                    store.clone(),
                    block,
                    sender,
                    &payload,
                    GenericResult::Ok(submsg_events),
                )?);
            },
            // error - callback requested
            // discard uncommitted state changes, give callback
            (ReplyOn::Error(payload) | ReplyOn::Always(payload), Result::Err(err)) => {
                events.extend(do_reply(
                    store.clone(),
                    block,
                    sender,
                    &payload,
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

pub fn do_reply<S: Storage + Clone + 'static>(
    store:      S,
    block:      &BlockInfo,
    contract:   &Addr,
    payload:    &Json,
    submsg_res: SubMsgResult,
) -> AppResult<Vec<Event>> {
    match _do_reply(store, block, contract, payload, submsg_res) {
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

fn _do_reply<S: Storage + Clone + 'static>(
    store:      S,
    block:      &BlockInfo,
    contract:   &Addr,
    payload:    &Json,
    submsg_res: SubMsgResult,
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
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        contract.clone(),
        sender:          None,
        funds:           None,
        simulate:        None,
    };
    let resp = instance.call_reply(&ctx, payload, &submsg_res)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_reply_event(contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, contract, resp.submsgs)?);

    Ok(events)
}
