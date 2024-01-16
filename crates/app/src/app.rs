use {
    crate::{auth::authenticate_tx, execute::process_msg, query::process_query},
    anyhow::{anyhow, ensure},
    cw_db::{Batch, CacheStore, Flush, SharedStore},
    cw_std::{
        Account, Addr, Binary, BlockInfo, Config, GenesisState, Hash, Item, Map, QueryRequest,
        QueryResponse, Storage, Tx,
    },
    tracing::{debug, info},
};

pub const CONFIG:               Item<Config>        = Item::new("config");
pub const LAST_FINALIZED_BLOCK: Item<BlockInfo>     = Item::new("last_finalized_block");
pub const CODES:                Map<&Hash, Binary>  = Map::new("c");
pub const ACCOUNTS:             Map<&Addr, Account> = Map::new("a");
pub const CONTRACT_NAMESPACE:   &[u8]               = b"w";

pub struct App<S> {
    store:         SharedStore<S>,
    pending:       Option<Batch>,
    current_block: Option<BlockInfo>,
}

impl<S> App<S> {
    pub fn new(store: S) -> Self {
        Self {
            store:         SharedStore::new(store),
            pending:       None,
            current_block: None,
        }
    }

    fn take_pending(&mut self) -> anyhow::Result<Batch> {
        self.pending.take().ok_or(anyhow!("[App]: pending batch not found"))
    }

    fn take_current_block(&mut self) -> anyhow::Result<BlockInfo> {
        self.current_block.take().ok_or(anyhow!("[App]: current block info not found"))
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
        // TODO: find value for height and timestamp here
        let block = BlockInfo {
            chain_id:  genesis_state.chain_id.clone(),
            height:    0,
            timestamp: 0,
        };

        // save the config and genesis block. some genesis messages may need it
        CONFIG.save(&mut self.store, &genesis_state.config)?;
        LAST_FINALIZED_BLOCK.save(&mut self.store, &block)?;

        // not sure which address to use as genesis message sender. currently we
        // just use an all-zero address.
        // probably should make the sender Option in the contexts. None if it's
        // in genesis.
        let sender = Addr::mock(0);

        // loop through genesis messages and execute each one.
        // it's expected that genesis messages should all successfully execute.
        // if anyone fails, it's fatal error and we abort the genesis.
        // the developer should examine the error, fix it, and retry.
        for (idx, msg) in genesis_state.msgs.into_iter().enumerate() {
            debug!(idx, "processing genesis message");
            process_msg(self.store.share(), &block, &sender, msg)?;
        }

        info!(chain_id = genesis_state.chain_id, "genesis complete");

        Ok(())
    }

    pub fn finalize_block(&mut self, block: BlockInfo, txs: Vec<Tx>) -> anyhow::Result<()> {
        let cached = SharedStore::new(CacheStore::new(self.store.share(), self.pending.take()));

        for (idx, tx) in txs.into_iter().enumerate() {
            // TODO: add txhash to the debug print
            debug!(idx, "processing tx");
            run_tx(cached.share(), &block, tx)?;
        }

        let (_, pending) = cached.disassemble()?.disassemble();

        self.put_pending(pending)?;
        self.put_current_block(block.clone())?;

        info!(height = block.height, timestamp = block.timestamp, "finalized block");

        Ok(())
    }

    pub fn query(&mut self, req: QueryRequest) -> anyhow::Result<QueryResponse> {
        // note: when doing query, we use the state from the last finalized block,
        // do not include uncommitted changes from the current block.
        let block = LAST_FINALIZED_BLOCK.load(&self.store)?;

        process_query(self.store.share(), &block, req)
    }
}

impl<S> App<S>
where
    S: Storage + Flush + 'static,
{
    // TODO: we need to think about what to do if the flush fails here...
    pub fn commit(&mut self) -> anyhow::Result<()> {
        let pending = self.take_pending()?;
        let current_block = self.take_current_block()?;

        // apply the DB ops effected by txs in this block
        self.store.flush(pending)?;

        // update the last finalized block info
        LAST_FINALIZED_BLOCK.save(&mut self.store, &current_block)?;

        info!(height = current_block.height, "committed state deltas");

        Ok(())
    }
}

fn run_tx<S>(store: S, block: &BlockInfo, tx: Tx) -> anyhow::Result<()>
where
    S: Storage + Flush + 'static,
{
    // create cached store for this tx
    let cached = SharedStore::new(CacheStore::new(store, None));

    // first, authenticate tx by calling the sender account's before_tx method
    if authenticate_tx(cached.share(), block, &tx).is_err() {
        // if authentication fails, abort, discard uncommitted changes
        return Ok(());
    }

    // update the account state. as long as authentication succeeds, regardless
    // of whether the message are successful, we update account state. if auth
    // fails, we don't update account state.
    cached.borrow_mut().commit()?;

    // now that the tx is authenticated, we loop through the messages and
    // execute them one by one
    for (idx, msg) in tx.msgs.into_iter().enumerate() {
        debug!(idx, "processing msg");
        if process_msg(cached.share(), block, &tx.sender, msg).is_err() {
            // if any one of the msgs fails, the entire tx fails.
            // abort, discard uncommitted changes (the changes from the before_tx
            // call earlier are persisted)
            return Ok(());
        }
    }

    // TODO: add `after_tx` hook?

    // all messages succeeded. commit the state changes
    cached.borrow_mut().commit()?;

    Ok(())
}
