//! The lifecycle of `S`:
//!
//! App takes a generic S which implements the Storage trait. For production,
//! this should be a wrapper around a RocksDB instance. For testing, this should
//! be cw_std::MockStorage.
//!
//! When finalize_block is called, this is wrapped in CacheStore<S>.
//! We then loop through the txs in the block.
//!
//! For each tx, it is wrapped again: CacheStore<CacheStore<S>>.
//! We then loop through the messages in the tx.
//!
//! When processing the msg, if calling a Wasm instance is involved, it is
//! wrapped again: PrefixStore<CacheStore<CacheStore<S>>>.
//!
//! Whether a message is successful or not, we unwrap the PrefixStore and return
//! the underlying: CacheStore<CacheStore<S>>.
//!
//! If all messages are successful, we commit the pending ops. If any message
//! fails, we discard the pending ops. Either way, the cache is unwrapped:
//! CacheStore<S>.
//!
//! Back to the scope of finalize_block. After all txs have been processed, we
//! disassamble the cache store, and keep the S and pending batch in memory.
//!
//! When commit is called, we flush the pending ops into S.

use {
    crate::{Batch, CacheStore, Flush, PrefixStore},
    anyhow::{anyhow, ensure},
    cw_std::{
        hash, to_json, Account, AccountResponse, Addr, Binary, BlockInfo, Bound, Coin,
        GenesisState, Hash, InfoResponse, Item, Map, Message, Order, Query, Storage, Tx,
        WasmSmartResponse,
    },
    cw_vm::{db_next, db_read, db_remove, db_scan, db_write, debug, Host, InstanceBuilder},
    wasmi::{Instance, Store},
};

// storage types
const CHAIN_ID:             Item<String>        = Item::new("cid");
const LAST_FINALIZED_BLOCK: Item<BlockInfo>     = Item::new("lfb");
const CODES:                Map<&Hash, Binary>  = Map::new("c");
const ACCOUNTS:             Map<&Addr, Account> = Map::new("a");
const CONTRACT_NAMESPACE:   &[u8]               = b"w";

// pagination parameters
const DEFAULT_PAGE_LIMIT: u32 = 30;

pub struct App<S> {
    store:         Option<S>,
    pending:       Option<Batch>,
    current_block: Option<BlockInfo>,
}

impl<S> App<S> {
    pub fn new(store: S) -> Self {
        Self {
            store:         Some(store),
            pending:       None,
            current_block: None,
        }
    }

    fn take_store(&mut self) -> anyhow::Result<S> {
        self.store.take().ok_or(anyhow!("[App]: store not found"))
    }

    fn take_pending(&mut self) -> anyhow::Result<Batch> {
        self.pending.take().ok_or(anyhow!("[App]: pending batch not found"))
    }

    fn take_current_block(&mut self) -> anyhow::Result<BlockInfo> {
        self.current_block.take().ok_or(anyhow!("[App]: current block info not found"))
    }

    fn put_store(&mut self, store: S) -> anyhow::Result<()> {
        ensure!(self.store.is_none(), "[App]: store already exists");
        self.store = Some(store);
        Ok(())
    }

    fn put_pending(&mut self, pending: Batch) -> anyhow::Result<()> {
        ensure!(self.pending.is_none(), "[App]: pending batch already exists");
        self.pending = Some(pending);
        Ok(())
    }

    fn put_current_block(&mut self, current_block: BlockInfo) -> anyhow::Result<()> {
        ensure!(self.current_block.is_none(), "[App]: current block info already exists");
        self.current_block = Some(current_block);
        Ok(())
    }
}

impl<S> App<S>
where
    S: Storage + Flush + 'static,
{
    pub fn init_chain(&mut self, genesis_state: GenesisState) -> anyhow::Result<()> {
        let mut store = self.take_store()?;

        CHAIN_ID.save(&mut store, &genesis_state.chain_id)?;

        debug_assert!(genesis_state.msgs.is_empty(), "UNIMPLEMENTED: genesis msg is not supported yet");

        self.put_store(store)
    }

    pub fn finalize_block(&mut self, block: BlockInfo, txs: Vec<Tx>) -> anyhow::Result<()> {
        let store = self.take_store()?;

        // TODO: check block height and time is valid
        // height must be that of the last finalized block + 1
        // time must be greater than that of the last finalized block

        let mut cached = CacheStore::new(store, self.pending.take());

        for tx in txs {
            cached = run_tx(cached, tx)?;
        }

        let (store, pending) = cached.disassemble();

        self.put_store(store)?;
        self.put_pending(pending)?;
        self.put_current_block(block)
    }

    pub fn commit(&mut self) -> anyhow::Result<()> {
        let mut store = self.take_store()?;
        let pending = self.take_pending()?;
        let current_block = self.take_current_block()?;

        // apply the DB ops effected by txs in this block
        store.flush(pending)?;

        // update the last finalized block info
        LAST_FINALIZED_BLOCK.save(&mut store, &current_block)?;

        // put the store back
        self.put_store(store)
    }

    pub fn query(&mut self, req: Query) -> anyhow::Result<Binary> {
        let store = self.take_store()?;

        // perform the query
        let (res, store) = query(store, req);

        // put the store back
        self.put_store(store)?;

        res
    }
}

fn run_tx<S>(store: S, tx: Tx) -> anyhow::Result<S>
where
    S: Storage + Flush + 'static,
{
    // TODO: authenticate txs

    // create cached store for this tx
    // if execution fails, state changes won't be committed
    let mut result;
    let mut cached = CacheStore::new(store, None);

    for msg in tx.msgs {
        (result, cached) = run_msg(cached, msg);

        // if any one of the msgs fails, the entire tx fails.
        // discard uncommitted changes and return the underlying store
        if result.is_err() {
            let (store, _) = cached.disassemble();
            return Ok(store);
        }
    }

    // all messages succeeded. commit the state changes
    cached.flush()
}

// take an owned mutable Storage value and execute a message on it. return
// the Storage value and a result indicating whether the message was successful.
//
// we don't need to create a cached store for the msg, because tx execution is
// atomic - a single msg fails, the entire tx fails, and the cache created in
// run_tx will be discarded.
fn run_msg<S>(mut store: S, msg: Message) -> (anyhow::Result<()>, S)
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
        } => execute(store, contract, msg, funds),
    }
}

fn store_code<S: Storage>(store: &mut S, wasm_byte_code: &Binary) -> anyhow::Result<()> {
    // TODO: static check, ensure wasm code has necessary imports/exports
    let hash = hash(wasm_byte_code);
    let exists = CODES.has(store, &hash);
    ensure!(!exists, "Do not upload the same code twice");

    CODES.save(store, &hash, wasm_byte_code)
}

fn instantiate<S: Storage + 'static>(
    store:     S,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Vec<Coin>,
    admin:     Option<Addr>,
) -> (anyhow::Result<()>, S) {
    debug_assert!(funds.is_empty(), "UNIMPLEMENTED: sending funds is not supported yet");

    // load wasm code
    let wasm_byte_code = match CODES.load(&store, &code_hash) {
        Ok(wasm_byte_code) => wasm_byte_code,
        Err(err) => return (Err(err), store),
    };

    // compute contract address
    let contract = Addr::compute(&code_hash, &salt);

    // create wasm host
    let (instance, mut wasm_store) = must_build_wasm_instance(store, &contract, wasm_byte_code);
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
    if let Err(err) = ACCOUNTS.save(&mut store, &contract, &account) {
        return (Err(err), store);
    }

    (Ok(()), store)
}

fn execute<S: Storage + 'static>(
    store:     S,
    contract:  Addr,
    msg:       Binary,
    funds:     Vec<Coin>,
) -> (anyhow::Result<()>, S) {
    debug_assert!(funds.is_empty(), "UNIMPLEMENTED: sending funds is not supported yet");

    // load contract info
    let account = match ACCOUNTS.load(&store, &contract) {
        Ok(account) => account,
        Err(err) => return (Err(err), store),
    };

    // load wasm code
    let wasm_byte_code = match CODES.load(&store, &account.code_hash) {
        Ok(wasm_byte_code) => wasm_byte_code,
        Err(err) => return (Err(err), store),
    };

    // create wasm host
    let (instance, mut wasm_store) = must_build_wasm_instance(store, &contract, wasm_byte_code);
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

fn query<S: Storage + 'static>(store: S, req: Query) -> (anyhow::Result<Binary>, S) {
    match req {
        Query::Info {} => (query_info(&store), store),
        Query::Account {
            address,
        } => (query_account(&store, address), store),
        Query::Accounts {
            start_after,
            limit,
        } => (query_accounts(&store, start_after, limit), store),
        Query::WasmRaw {
            contract,
            key,
        } => (query_wasm_raw(&store, contract, key), store),
        Query::WasmSmart {
            contract,
            msg
        } => query_wasm_smart(store, contract, msg),
    }
}

fn query_info(store: &dyn Storage) -> anyhow::Result<Binary> {
    let resp = InfoResponse {
        chain_id: CHAIN_ID.load(store)?,
        last_finalized_block: LAST_FINALIZED_BLOCK.load(store)?,
    };
    to_json(&resp)
}

fn query_account(store: &dyn Storage, address: Addr) -> anyhow::Result<Binary> {
    let account = ACCOUNTS.load(store, &address)?;
    let resp = AccountResponse {
        address,
        code_hash: account.code_hash,
        admin:     account.admin,
    };
    to_json(&resp)
}

fn query_accounts(store: &dyn Storage, start_after: Option<Addr>, limit: Option<u32>) -> anyhow::Result<Binary> {
    let start = start_after.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    let resp = ACCOUNTS
        .range(store, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|item| {
            let (address, account) = item?;
            Ok(AccountResponse {
                address,
                code_hash: account.code_hash,
                admin:     account.admin,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    to_json(&resp)
}

fn query_wasm_raw(
    _store:    &dyn Storage,
    _contract: Addr,
    _key:      Binary,
) -> anyhow::Result<Binary> {
    todo!()
}

fn query_wasm_smart<S: Storage + 'static>(
    store:    S,
    contract: Addr,
    msg:      Binary,
) -> (anyhow::Result<Binary>, S) {
    // load contract info
    let account = match ACCOUNTS.load(&store, &contract) {
        Ok(account) => account,
        Err(err) => return (Err(err), store),
    };

    // load wasm code
    let wasm_byte_code = match CODES.load(&store, &account.code_hash) {
        Ok(wasm_byte_code) => wasm_byte_code,
        Err(err) => return (Err(err), store),
    };

    // create wasm host
    let (instance, mut wasm_store) = must_build_wasm_instance(store, &contract, wasm_byte_code);
    let mut host = Host::new(&instance, &mut wasm_store);

    // call query
    let data = match host.call_query(msg) {
        Ok(data) => data,
        Err(err) => {
            let store = wasm_store.into_data().disassemble();
            return (Err(err), store);
        },
    };

    let query_res = WasmSmartResponse {
        contract,
        data,
    };

    (to_json(&query_res), wasm_store.into_data().disassemble())
}

fn must_build_wasm_instance<S: Storage + 'static>(
    store: S,
    addr:  &Addr,
    wasm:  impl AsRef<[u8]>,
) -> (Instance, Store<PrefixStore<S>>) {
    build_wasm_instance(store, addr, wasm)
        .unwrap_or_else(|err| panic!("Fatal error! Failed to build wasm instance: {err}"))
}

fn build_wasm_instance<S: Storage + 'static>(
    store: S,
    addr:  &Addr,
    wasm:  impl AsRef<[u8]>,
) -> anyhow::Result<(Instance, Store<PrefixStore<S>>)> {
    InstanceBuilder::default()
        .with_wasm_bytes(wasm)?
        .with_storage(PrefixStore::new(store, &[CONTRACT_NAMESPACE, addr.as_ref()]))
        .with_host_function("db_read", db_read)?
        .with_host_function("db_write", db_write)?
        .with_host_function("db_remove", db_remove)?
        .with_host_function("db_scan", db_scan)?
        .with_host_function("db_next", db_next)?
        .with_host_function("debug", debug)?
        .finalize()
}
