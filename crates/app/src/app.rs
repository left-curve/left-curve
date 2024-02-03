use {
    crate::{
        authenticate_tx, process_msg, process_query, AppError, AppResult, CHAIN_ID, CONFIG,
        LAST_FINALIZED_BLOCK,
    },
    cw_db::{CacheStore, SharedStore},
    cw_std::{
        from_json, to_json, Addr, Batch, Binary, BlockInfo, Event, GenesisState, Hash,
        QueryRequest, Storage, Tx, Uint64,
    },
    std::sync::{Arc, RwLock},
    tracing::{debug, info},
};

/// Represent state changes caused by the FinalizeBlock call, but not yet
/// persisted to disk.
struct PendingData {
    /// Database write batch
    batch: Batch,
    /// The unfinalized block
    block: BlockInfo,
}

/// The ABCI application.
#[derive(Clone)]
pub struct App<S> {
    /// The underlying database. Must be thread safe.
    store: SharedStore<S>,
    /// State changes that have not yet been persisted to disk.
    pending: Arc<RwLock<Option<PendingData>>>,
}

impl<S> App<S> {
    pub fn new(store: S) -> Self {
        Self {
            store:   SharedStore::new(store),
            pending: Arc::new(RwLock::new(None)),
        }
    }

    fn take_pending(&self) -> AppResult<(Batch, BlockInfo)> {
        let mut lock = self.pending.write().map_err(|_| AppError::PendingDataPoisoned)?;
        let data = lock.take().ok_or(AppError::PendingDataNotSet)?;
        Ok((data.batch, data.block))
    }

    fn put_pending(&self, batch: Batch, block: BlockInfo) -> AppResult<()> {
        let mut lock = self.pending.write().map_err(|_| AppError::PendingDataPoisoned)?;
        if lock.replace(PendingData { batch, block }).is_some() {
            return Err(AppError::PendingDataExists);
        }
        Ok(())
    }
}

impl<S> App<S>
where
    S: Storage + 'static,
{
    pub fn do_init_chain(
        &self,
        chain_id: String,
        block: BlockInfo,
        app_state_bytes: &[u8],
    ) -> AppResult<Hash> {
        let mut store = self.store.share();

        // deserialize the genesis state
        let genesis_state: GenesisState = from_json(app_state_bytes)?;

        // save the config and genesis block. some genesis messages may need it
        CHAIN_ID.save(&mut store, &chain_id)?;
        CONFIG.save(&mut store, &genesis_state.config)?;
        LAST_FINALIZED_BLOCK.save(&mut store, &block)?;

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
            info!(idx, "Processing genesis message");
            process_msg(self.store.share(), &block, &sender, msg)?;
        }

        info!(chain_id, "Completed genesis");

        // return an empty apphash as placeholder, since we haven't implemented
        // state merklization yet
        Ok(Hash::zero())
    }

    // TODO: return events, txResults, appHash
    pub fn do_finalize_block(
        &self,
        block:   BlockInfo,
        raw_txs: Vec<impl AsRef<[u8]>>,
    ) -> AppResult<Vec<AppResult<Vec<Event>>>> {
        let cached = SharedStore::new(CacheStore::new(self.store.share(), None));
        let mut tx_results = vec![];

        for (idx, raw_tx) in raw_txs.into_iter().enumerate() {
            // TODO: add txhash to the debug print
            debug!(idx, "Processing transaction");
            tx_results.push(run_tx(cached.share(), &block, from_json(raw_tx)?));
        }

        let (_, batch) = cached.disassemble().disassemble();

        self.put_pending(batch, block.clone())?;

        info!(height = block.height.u64(), timestamp = block.timestamp.seconds(), "Finalized block");

        Ok(tx_results)
    }

    // TODO: we need to think about what to do if the flush fails here?
    pub fn do_commit(&self) -> AppResult<()> {
        let mut store = self.store.share();
        let (batch, block) = self.take_pending()?;

        // apply the DB ops effected by txs in this block
        store.flush(batch);

        // update the last finalized block info
        LAST_FINALIZED_BLOCK.save(&mut store, &block)?;

        info!(height = block.height.u64(), "Committed state");

        Ok(())
    }

    // returns (last_block_height, last_block_app_hash)
    pub fn do_info(&self) -> AppResult<(Uint64, Hash)> {
        match LAST_FINALIZED_BLOCK.may_load(&self.store)? {
            Some(block) => {
                // return an all-zero hash as a placeholder, since we haven't
                // implemented state merklization yet
                Ok((block.height, Hash::zero()))
            },
            None => {
                // prior to genesis, we simply return 0 as block height and an
                // empty app hash
                Ok((Uint64::zero(), Hash::zero()))
            },
        }
    }

    pub fn do_query_app(&self, raw_query: &[u8]) -> AppResult<Binary> {
        // note: when doing query, we use the state from the last finalized block,
        // do not include uncommitted changes from the current block.
        let block = LAST_FINALIZED_BLOCK.load(&self.store)?;

        let req: QueryRequest = from_json(raw_query)?;
        let res = process_query(self.store.share(), &block, req)?;

        to_json(&res).map_err(Into::into)
    }

    pub fn do_query_store(
        &self,
        key:    &[u8],
        height: u64,
        prove:  bool,
    ) -> Option<Vec<u8>> { // TODO: add proof to return data
        debug_assert_eq!(height, 0, "query at past height isn't supported yet");
        debug_assert!(!prove, "merkle proof isn't supported yet");

        self.store.read_access().read(key)
    }
}

fn run_tx<S>(store: S, block: &BlockInfo, tx: Tx) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
{
    let mut events = vec![];

    // create cached store for this tx
    let cached = SharedStore::new(CacheStore::new(store, None));

    // first, authenticate tx by calling the sender account's before_tx method.
    // if authentication fails, abort, discard uncommitted.
    events.extend(authenticate_tx(cached.share(), block, &tx)?);

    // update the account state. as long as authentication succeeds, regardless
    // of whether the message are successful, we update account state. if auth
    // fails, we don't update account state.
    cached.write_access().commit();

    // now that the tx is authenticated, we loop through the messages and
    // execute them one by one.
    // if any one of the msgs fails, the entire tx fails; abort, discard
    // uncommitted changes (the changes from the before_tx call earlier are
    // persisted)
    for (idx, msg) in tx.msgs.into_iter().enumerate() {
        debug!(idx, "Processing message");
        events.extend(process_msg(cached.share(), block, &tx.sender, msg)?);
    }

    // all messages succeeded. commit the state changes
    cached.write_access().commit();

    Ok(events)
}
