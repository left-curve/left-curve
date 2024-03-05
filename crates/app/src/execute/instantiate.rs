use {
    super::{handle_submessages, has_permission, new_instantiate_event, transfer},
    crate::{AppError, AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{Account, Addr, Binary, BlockInfo, Coins, Context, Event, Hash, Storage},
    cw_vm::Instance,
    tracing::{info, warn},
};

#[allow(clippy::too_many_arguments)]
pub fn instantiate<S: Storage + Clone + 'static>(
    store:     S,
    block:     &BlockInfo,
    sender:    &Addr,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Coins,
    admin:     Option<Addr>,
) -> AppResult<Vec<Event>> {
    match _instantiate(store, block, sender, code_hash, msg, salt, funds, admin) {
        Ok((events, address)) => {
            info!(address = address.to_string(), "Instantiated contract");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to instantiate contract");
            Err(err)
        },
    }
}

// return the address of the contract that is instantiated.
#[allow(clippy::too_many_arguments)]
fn _instantiate<S: Storage + Clone + 'static>(
    mut store: S,
    block:     &BlockInfo,
    sender:    &Addr,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Coins,
    admin:     Option<Addr>,
) -> AppResult<(Vec<Event>, Addr)> {
    // make sure the user has permission to instantiate contracts
    let cfg = CONFIG.load(&store)?;
    if !has_permission(&cfg.instantiate_permission, cfg.owner.as_ref(), sender) {
        return Err(AppError::Unauthorized);
    }

    // compute contract address and make sure there can't already be an account
    // of the same address
    let address = Addr::compute(sender, &code_hash, &salt);
    if ACCOUNTS.has(&store, &address) {
        return Err(AppError::account_exists(address));
    }

    // load wasm code, make sure code exists
    let wasm_byte_code = CODES.load(&store, &code_hash)?;

    // save the account info now that we know there's no duplicate
    let account = Account { code_hash, admin };
    ACCOUNTS.save(&mut store, &address, &account)?;

    // make the coin transfers
    if !funds.is_empty() {
        transfer(
            store.clone(),
            block,
            sender.clone(),
            address.clone(),
            funds.clone(),
            false,
        )?;
    }

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &address]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call instantiate
    let ctx = Context {
        chain_id:        CHAIN_ID.load(&store)?,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        address,
        sender:          Some(sender.clone()),
        funds:           Some(funds),
        simulate:        None,
    };
    let resp = instance.call_instantiate(&ctx, msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_instantiate_event(&ctx.contract, &account.code_hash, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok((events, ctx.contract))
}
