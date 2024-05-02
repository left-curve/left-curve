use {
    crate::{
        create_vm_instance, handle_submessages, load_program, new_receive_event,
        new_transfer_event, AppError, AppResult, ACCOUNTS, CHAIN_ID, CONFIG,
    },
    cw_types::{Addr, BlockInfo, Coins, Context, Event, Storage, TransferMsg, Vm},
    tracing::{info, warn},
};

pub fn do_transfer<S, VM>(
    store:   S,
    block:   &BlockInfo,
    from:    Addr,
    to:      Addr,
    coins:   Coins,
    receive: bool,
) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_transfer::<S, VM>(store, block, from, to, coins, receive) {
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
fn _do_transfer<S, VM>(
    store:   S,
    block:   &BlockInfo,
    from:    Addr,
    to:      Addr,
    coins:   Coins,
    receive: bool,
) -> AppResult<(Vec<Event>, TransferMsg)>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&store)?;
    let cfg = CONFIG.load(&store)?;
    let account = ACCOUNTS.load(&store, &cfg.bank)?;

    let program = load_program::<VM>(&store, &account.code_hash)?;
    let mut instance = create_vm_instance::<S, VM>(store.clone(), block.clone(), &cfg.bank, program)?;

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
    };
    let msg = TransferMsg {
        from,
        to,
        coins,
    };
    let resp = instance.call_bank_transfer(&ctx, &msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_transfer_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages::<VM>(Box::new(store.clone()), block, &ctx.contract, resp.submsgs)?);

    if receive {
        // call the recipient contract's `receive` entry point to inform it of
        // this transfer. we do this when handing the Message::Transfer.
        _do_receive::<_, VM>(store, block, msg, events)
    } else {
        // do not call the `receive` entry point. we do this when handling
        // Message::Instantiate and Execute.
        Ok((events, msg))
    }
}

fn _do_receive<S, VM>(
    store:      S,
    block:      &BlockInfo,
    msg:        TransferMsg,
    mut events: Vec<Event>,
) -> AppResult<(Vec<Event>, TransferMsg)>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &msg.to)?;

    let program = load_program::<VM>(&store, &account.code_hash)?;
    let mut instance = create_vm_instance::<S, VM>(store.clone(), block.clone(), &msg.to, program)?;

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
    };
    let resp = instance.call_receive(&ctx)?.into_std_result()?;

    // handle submessages
    events.push(new_receive_event(&msg.to, resp.attributes));
    events.extend(handle_submessages::<VM>(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok((events, msg))
}
