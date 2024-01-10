use {
    crate::{
        execute::{authenticate_tx, process_msg},
        query::process_query,
    },
    anyhow::{anyhow, ensure},
    cw_db::{Batch, CacheStore, Flush},
    cw_std::{
        Account, Addr, Binary, BlockInfo, GenesisState, Hash, Item, Map, Query, QueryResponse,
        Storage, Tx,
    },
    tracing::{debug, error, info},
};

pub(crate) const LAST_FINALIZED_BLOCK: Item<BlockInfo>     = Item::new("lfb");
pub(crate) const CODES:                Map<&Hash, Binary>  = Map::new("c");
pub(crate) const ACCOUNTS:             Map<&Addr, Account> = Map::new("a");
pub(crate) const CONTRACT_NAMESPACE:   &[u8]               = b"w";

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
    S: Storage + 'static,
{
    pub fn init_chain(&mut self, genesis_state: GenesisState) -> anyhow::Result<()> {
        // we don't use a cache here, because if a genesis message fails to
        // execute, it's a fatal error, we should panic and reset.
        let mut store = self.take_store()?;

        // TODO: find value for height and timestamp here
        let block = BlockInfo {
            chain_id:  genesis_state.chain_id.clone(),
            height:    0,
            timestamp: 0,
        };

        // not sure which address to use as genesis message sender. currently we
        // just use an all-zero address.
        // probably should make the sender Option in the contexts. None if it's
        // in genesis.
        let sender = Addr::mock(0);

        let mut result;
        for (idx, msg) in genesis_state.msgs.into_iter().enumerate() {
            debug!(idx, "processing genesis message");

            (result, store) = process_msg(store, &block, &sender, msg);

            if result.is_err() {
                error!("error processing genesis messages");
                return result;
            }
        }

        // save the genesis block
        LAST_FINALIZED_BLOCK.save(&mut store, &block)?;

        // put store back
        self.put_store(store)?;

        info!(chain_id = genesis_state.chain_id, "genesis complete");

        Ok(())
    }

    pub fn finalize_block(&mut self, block: BlockInfo, txs: Vec<Tx>) -> anyhow::Result<()> {
        let store = self.take_store()?;

        // TODO: check block height and time is valid
        // height must be that of the last finalized block + 1
        // time must be greater than that of the last finalized block

        let mut cached = CacheStore::new(store, self.pending.take());

        for (idx, tx) in txs.into_iter().enumerate() {
            // TODO: add txhash to the debug print
            debug!(idx, "processing tx");
            cached = run_tx(cached, &block, tx)?;
        }

        let (store, pending) = cached.disassemble();

        self.put_store(store)?;
        self.put_pending(pending)?;
        self.put_current_block(block.clone())?;

        info!(height = block.height, timestamp = block.timestamp, "finalized block");

        Ok(())
    }

    pub fn query(&mut self, req: Query) -> anyhow::Result<QueryResponse> {
        // note: when doing query, we use the state from the last finalized block,
        // do not include uncommitted changes from the current block.
        let store = self.take_store()?;
        let block = LAST_FINALIZED_BLOCK.load(&store)?;

        // perform the query
        let (res, store) = process_query(store, &block, req);

        // put the store back
        self.put_store(store)?;

        res
    }
}

impl<S> App<S>
where
    S: Storage + Flush + 'static,
{
    pub fn commit(&mut self) -> anyhow::Result<()> {
        let mut store = self.take_store()?;
        let pending = self.take_pending()?;
        let current_block = self.take_current_block()?;

        // apply the DB ops effected by txs in this block
        store.flush(pending)?;

        // update the last finalized block info
        LAST_FINALIZED_BLOCK.save(&mut store, &current_block)?;

        // put the store back
        self.put_store(store)?;

        info!(height = current_block.height, "committed state deltas");

        Ok(())
    }
}

fn run_tx<S>(store: S, block: &BlockInfo, tx: Tx) -> anyhow::Result<S>
where
    S: Storage + Flush + 'static,
{
    // create cached store for this tx
    let mut cached = CacheStore::new(store, None);
    let mut result;

    // first, authenticate the tx by calling the sender account's `before_tx`
    // entry point.
    (result, cached) = authenticate_tx(cached, block, &tx);
    if result.is_err() {
        let (store, _) = cached.disassemble();
        return Ok(store);
    }

    // update the account state. as long as authentication succeeds, regardless
    // of whether the message are successful, we update account state. if auth
    // fails, we don't update account state.
    cached.flush()?;

    // now that the tx is authenticated, we loop through the messages and
    // execute them one by one
    for (idx, msg) in tx.msgs.into_iter().enumerate() {
        debug!(idx, "processing msg");

        (result, cached) = process_msg(cached, block, &tx.sender, msg);
        if result.is_err() {
            // if any one of the msgs fails, the entire tx fails.
            // discard uncommitted changes and return the underlying store.
            let (store, _) = cached.disassemble();
            return Ok(store);
        }
    }

    // TODO: add `after_tx` hook?

    // all messages succeeded. commit the state changes
    cached.flush_to_base()
}
