use {
    anyhow::{anyhow, ensure},
    cw_std::{
        from_json, hash, Account, Addr, Batch, Binary, BlockInfo, CacheStore, ContractResult,
        GenesisState, Hash, Item, Map, Message, MockStorage, Query, Response, Storage, Tx,
    },
    cw_vm::{
        db_next, db_read, db_remove, db_scan, db_write, debug, Host, HostState, InstanceBuilder,
    },
    std::collections::HashMap,
    wasmi::{Instance, Store},
};

const LAST_FINALIZED_BLOCK: Item<BlockInfo> = Item::new("lfb");
const CODES: Map<&Hash, Binary> = Map::new("c");
const ACCOUNTS: Map<&Addr, Account> = Map::new("a");

pub struct App<S> {
    chain_id:      String,
    store:         Option<S>,
    pending:       Option<Batch>,
    current_block: Option<BlockInfo>,

    // TODO: these should be a prefixed store based on self.store
    contract_stores: HashMap<Addr, MockStorage>,
}

impl<S> App<S> {
    pub fn new(chain_id: impl Into<String>, store: S) -> Self {
        Self {
            chain_id:        chain_id.into(),
            store:           Some(store),
            pending:         None,
            current_block:   None,
            contract_stores: HashMap::new(),
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
    S: Storage + 'static,
{
    pub fn init_chain(&mut self, _genesis_state: GenesisState) -> anyhow::Result<()> {
        todo!()
    }

    pub fn finalize_block(&mut self, block: BlockInfo, txs: Vec<Tx>) -> anyhow::Result<()> {
        let store = self.take_store()?;
        let mut cached = CacheStore::new(store, self.pending.take());

        for tx in txs {
            cached = self.run_tx(cached, tx)?;
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
        store.apply(pending)?;

        // update the last finalized block info
        LAST_FINALIZED_BLOCK.save(&mut store, &current_block)?;

        // put the store back
        self.put_store(store)
    }

    pub fn query(&self, _req: Query) -> anyhow::Result<Binary> {
        todo!()
    }
}

fn run_tx<S>(mut store: S, tx: Tx) -> anyhow::Result<S>
where
    S: Storage + 'static,
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
    cached.commit()
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
        } => {
            let hash = hash(&wasm_byte_code);
            let exists = CODES.has(&store, &hash)?;
            ensure!(!exists, "Please don't upload the same code twice");

            CODES.save(&mut store, &hash, &wasm_byte_code)?;

            Ok(())
        },
        Message::Instantiate {
            code_hash,
            msg,
            salt,
            funds,
            admin,
        } => {
            ensure!(funds.is_empty(), "UNIMPLEMENTED: sending funds is not supported yet");

            // load wasm code
            let wasm_byte_code = CODES.load(&store, &code_hash)?;

            // compute contract address
            let address = Addr::compute(&code_hash, &salt);

            // create wasm host
            let (instance, mut wasm_store) = build_wasm_host(store, &wasm_byte_code)?;
            let mut host = Host::new(&instance, &mut wasm_store);

            // call instantiate
            let res_bytes = host.call_entry_point_raw("instantiate", msg)?;
            let res: ContractResult<Response> = from_json(&res_bytes)?;
            let resp = res.into_result()?;

            ensure!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

            // save account info
            let mut store = wasm_store.into_data().disassemble();
            let account = Account {
                code_hash,
                admin,
            };
            ACCOUNTS.save(&mut store, &address, &account)?;

            Ok(())
        },
        Message::Execute {
            contract,
            msg,
            funds,
        } => {
            ensure!(funds.is_empty(), "UNIMPLEMENTED: sending funds is not supported yet");

            // load contract info
            let account = ACCOUNTS.load(&store, &contract)?;

            // load wasm code
            let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

            // create wasm host
            let (instance, mut wasm_store) = build_wasm_host(store, &wasm_byte_code)?;
            let mut host = Host::new(&instance, &mut wasm_store);

            // call execute
            let res_bytes = host.call_entry_point_raw("execute", msg)?;
            let res: ContractResult<Response> = from_json(&res_bytes)?;
            let resp = res.into_result()?;

            ensure!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

            Ok(())
        },
    }
}

fn build_wasm_host<S: Storage + 'static>(
    store: S,
    wasm_byte_code: impl AsRef<[u8]>,
) -> anyhow::Result<(Instance, Store<HostState<S>>)> {
    InstanceBuilder::default()
        .with_wasm_bytes(wasm_byte_code)?
        .with_storage(store)
        .with_host_function("db_read", db_read)?
        .with_host_function("db_write", db_write)?
        .with_host_function("db_remove", db_remove)?
        .with_host_function("db_scan", db_scan)?
        .with_host_function("db_next", db_next)?
        .with_host_function("debug", debug)?
        .finalize()
}
