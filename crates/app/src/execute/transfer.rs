use {
    super::{handle_submessages, new_receive_event, new_transfer_event},
    crate::{AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{Addr, BlockInfo, Coins, Context, Event, Storage, TransferMsg},
    cw_vm::Instance,
    tracing::{info, warn},
};

pub fn transfer<S: Storage + Clone + 'static>(
    store:   S,
    block:   &BlockInfo,
    from:    Addr,
    to:      Addr,
    coins:   Coins,
    receive: bool,
) -> AppResult<Vec<Event>> {
    match _transfer(store, block, from, to, coins, receive) {
        Ok((events, msg)) => {
            info!(
                from  = msg.from.to_string(),
                to    = msg.to.to_string(),
                coins = msg.coins.to_string(),
                "Transferred coins"
            );
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to transfer coins");
            Err(err)
        },
    }
}

// return the TransferMsg, which includes the sender, receiver, and amount, for
// purpose of tracing/logging
fn _transfer<S: Storage + Clone + 'static>(
    store:   S,
    block:   &BlockInfo,
    from:    Addr,
    to:      Addr,
    coins:   Coins,
    receive: bool,
) -> AppResult<(Vec<Event>, TransferMsg)> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let cfg = CONFIG.load(&store)?;
    let account = ACCOUNTS.load(&store, &cfg.bank)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &cfg.bank]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call transfer
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        cfg.bank,
        sender:          None,
        funds:           None,
        simulate:        None,
        submsg_result:   None,
    };
    let msg = TransferMsg {
        from,
        to,
        coins,
    };
    let resp = instance.call_transfer(&ctx, &msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_transfer_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store.clone()), block, &ctx.contract, resp.submsgs)?);

    if receive {
        // call the recipient contract's `receive` entry point to inform it of
        // this transfer. we do this when handing the Message::Transfer.
        _receive(store, block, msg, events)
    } else {
        // do not call the `receive` entry point. we do this when handling
        // Message::Instantiate and Execute.
        Ok((events, msg))
    }
}

fn _receive<S: Storage + Clone + 'static>(
    store:      S,
    block:      &BlockInfo,
    msg:        TransferMsg,
    mut events: Vec<Event>,
) -> AppResult<(Vec<Event>, TransferMsg)> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &msg.to)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &msg.to]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call the recipient contract's `receive` entry point
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        msg.to.clone(),
        sender:          Some(msg.from.clone()),
        funds:           Some(msg.coins.clone()),
        simulate:        None,
        submsg_result:   None,
    };
    let resp = instance.call_receive(&ctx)?.into_std_result()?;

    // handle submessages
    events.push(new_receive_event(&msg.to, resp.attributes));
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok((events, msg))
}
