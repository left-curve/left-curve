use {
    anyhow::{anyhow, ensure},
    super::{CODES, ACCOUNTS, CONTRACT_NAMESPACE},
    crate::wasm::must_build_wasm_instance,
    cw_std::{Message, Storage, Binary, Hash, hash, Addr, Coin, Account},
    tracing::{info, warn},
    cw_vm::Host,
};

pub fn process_msg<S>(mut store: S, msg: Message) -> (anyhow::Result<()>, S)
where
    S: Storage + 'static,
{
    match msg {
        Message::StoreCode {
            wasm_byte_code,
        } => (store_code(&mut store, &wasm_byte_code), store),
        Message::Instantiate {
            code_hash,
            msg,
            salt,
            funds,
            admin,
        } => instantiate(store, code_hash, msg, salt, funds, admin),
        Message::Execute {
            contract,
            msg,
            funds,
        } => execute(store, &contract, msg, funds),
    }
}

// -------------------------------- store code ---------------------------------

fn store_code(store: &mut dyn Storage, wasm_byte_code: &Binary) -> anyhow::Result<()> {
    match _store_code(store, wasm_byte_code) {
        Ok(code_hash) => {
            info!(code_hash = code_hash.to_string(), "stored code");
            Ok(())
        },
        Err(err) => {
            warn!(err = err.to_string(), "failed to store code");
            Err(err)
        },
    }
}

fn _store_code(store: &mut dyn Storage, wasm_byte_code: &Binary) -> anyhow::Result<Hash> {
    // TODO: static check, ensure wasm code has necessary imports/exports
    let code_hash = hash(wasm_byte_code);

    ensure!(!CODES.has(store, &code_hash), "code with hash `{code_hash}` already exists");

    CODES.save(store, &code_hash, wasm_byte_code)?;

    Ok(code_hash)
}

// -------------------------------- instantiate --------------------------------

fn instantiate<S: Storage + 'static>(
    store:     S,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Vec<Coin>,
    admin:     Option<Addr>,
) -> (anyhow::Result<()>, S) {
    match _instantiate(store, code_hash, msg, salt, funds, admin) {
        (Ok(address), store) => {
            info!(address = address.to_string(), "instantiated contract");
            (Ok(()), store)
        },
        (Err(err), store) => {
            warn!(err = err.to_string(), "failed to instantiate contract");
            (Err(err), store)
        },
    }
}

fn _instantiate<S: Storage + 'static>(
    store:     S,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Vec<Coin>,
    admin:     Option<Addr>,
) -> (anyhow::Result<Addr>, S) {
    debug_assert!(funds.is_empty(), "UNIMPLEMENTED: sending funds is not supported yet");

    // load wasm code
    let wasm_byte_code = match CODES.load(&store, &code_hash) {
        Ok(wasm_byte_code) => wasm_byte_code,
        Err(err) => return (Err(err), store),
    };

    // compute contract address
    let address = Addr::compute(&code_hash, &salt);
    if ACCOUNTS.has(&store, &address) {
        return (Err(anyhow!("account with the address `{address}` already exists")), store);
    }

    // create wasm host
    let (instance, mut wasm_store) = must_build_wasm_instance(
        store,
        CONTRACT_NAMESPACE,
        &address,
        wasm_byte_code,
    );
    let mut host = Host::new(&instance, &mut wasm_store);

    // call instantiate
    let resp = match host.call_instantiate(msg) {
        Ok(resp) => resp,
        Err(err) => {
            let store = wasm_store.into_data().disassemble();
            return (Err(err), store);
        },
    };

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    // save account info
    let mut store = wasm_store.into_data().disassemble();
    let account = Account {
        code_hash,
        admin,
    };
    if let Err(err) = ACCOUNTS.save(&mut store, &address, &account) {
        return (Err(err), store);
    }

    (Ok(address), store)
}

// ---------------------------------- execute ----------------------------------

fn execute<S: Storage + 'static>(
    store:     S,
    contract:  &Addr,
    msg:       Binary,
    funds:     Vec<Coin>,
) -> (anyhow::Result<()>, S) {
    match _execute(store, contract, msg, funds) {
        (Ok(()), store) => {
            info!(contract = contract.to_string(), "executed contract");
            (Ok(()), store)
        },
        (Err(err), store) => {
            warn!(err = err.to_string(), "failed to execute contract");
            (Err(err), store)
        },
    }
}

fn _execute<S: Storage + 'static>(
    store:     S,
    contract:  &Addr,
    msg:       Binary,
    funds:     Vec<Coin>,
) -> (anyhow::Result<()>, S) {
    debug_assert!(funds.is_empty(), "UNIMPLEMENTED: sending funds is not supported yet");

    // load contract info
    let account = match ACCOUNTS.load(&store, contract) {
        Ok(account) => account,
        Err(err) => return (Err(err), store),
    };

    // load wasm code
    let wasm_byte_code = match CODES.load(&store, &account.code_hash) {
        Ok(wasm_byte_code) => wasm_byte_code,
        Err(err) => return (Err(err), store),
    };

    // create wasm host
    let (instance, mut wasm_store) = must_build_wasm_instance(
        store,
        CONTRACT_NAMESPACE,
        contract,
        wasm_byte_code,
    );
    let mut host = Host::new(&instance, &mut wasm_store);

    // call execute
    let resp = match host.call_execute(msg) {
        Ok(resp) => resp,
        Err(err) => {
            let store = wasm_store.into_data().disassemble();
            return (Err(err), store);
        },
    };

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    (Ok(()), wasm_store.into_data().disassemble())
}
