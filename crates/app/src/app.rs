use {
    crate::{
        after_block, after_tx, before_block, before_tx, process_msg, process_query, AppError,
        AppResult, CHAIN_ID, CONFIG, LAST_FINALIZED_BLOCK,
    },
    cw_db::{BaseStore, CacheStore, SharedStore},
    cw_std::{
        from_json, hash, to_json, Binary, BlockInfo, Event, GenesisState, Hash, QueryRequest,
        Storage, GENESIS_SENDER,
    },
    tracing::{debug, info},
};

/// The ABCI application.
///
/// Must be clonable which is required by `tendermint-abci` library:
/// https://github.com/informalsystems/tendermint-rs/blob/v0.34.0/abci/src/application.rs#L22-L25
#[derive(Clone)]
pub struct App {
    store: BaseStore,
}

impl App {
    pub fn new(store: BaseStore) -> Self {
        Self { store }
    }
}

impl App {
    pub fn do_init_chain(
        &self,
        chain_id:        String,
        block:           BlockInfo,
        app_state_bytes: &[u8],
    ) -> AppResult<Hash> {
        let mut cached = SharedStore::new(CacheStore::new(self.store.state_storage(None), None));

        // make sure the block height during InitChain is zero. this is necessary
        // to ensure that block height always matches the BaseStore version.
        if block.height.u64() != 0 {
            return Err(AppError::incorrect_block_height(0, block.height.u64()));
        }

        // deserialize the genesis state
        let genesis_state: GenesisState = from_json(app_state_bytes)?;

        // save the config and genesis block. some genesis messages may need it
        CHAIN_ID.save(&mut cached, &chain_id)?;
        CONFIG.save(&mut cached, &genesis_state.config)?;
        LAST_FINALIZED_BLOCK.save(&mut cached, &block)?;

        // loop through genesis messages and execute each one.
        // it's expected that genesis messages should all successfully execute.
        // if anyone fails, it's fatal error and we abort the genesis.
        // the developer should examine the error, fix it, and retry.
        for (idx, msg) in genesis_state.msgs.into_iter().enumerate() {
            info!(idx, "Processing genesis message");
            process_msg(cached.clone(), &block, &GENESIS_SENDER, msg)?;
        }

        // persist the state changes to disk
        let (_, pending) = cached.disassemble().disassemble();
        let (version, root_hash) = self.store.flush_and_commit(pending)?;

        // BaseStore version should be 0
        debug_assert_eq!(version, 0);
        // the root hash should not be None. it's only None when the merkle tree
        // is empty, but we have written some data to it (like the chain ID and
        // the config) so it shouldn't be empty.
        debug_assert!(root_hash.is_some());

        info!(
            chain_id,
            timestamp = block.timestamp.seconds(),
            app_hash  = root_hash.as_ref().unwrap().to_string(),
            "Completed genesis"
        );

        // return an empty apphash as placeholder, since we haven't implemented
        // state merklization yet
        Ok(root_hash.unwrap())
    }

    #[allow(clippy::type_complexity)]
    pub fn do_finalize_block(
        &self,
        block:   BlockInfo,
        raw_txs: Vec<impl AsRef<[u8]>>,
    ) -> AppResult<(Hash, Vec<Event>, Vec<AppResult<Vec<Event>>>)> {
        let mut cached = SharedStore::new(CacheStore::new(self.store.state_storage(None), None));
        let mut events = vec![];
        let mut tx_results = vec![];

        let cfg = CONFIG.load(&cached)?;
        let last_finalized_block = LAST_FINALIZED_BLOCK.load(&cached)?;

        // make sure the new block height is exactly the last finalized height
        // plus one. this ensures that block height always matches the BaseStore
        // version.
        if block.height.u64() != last_finalized_block.height.u64() + 1 {
            return Err(AppError::incorrect_block_height(
                last_finalized_block.height.u64() + 1,
                block.height.u64(),
            ));
        }

        // call begin blockers
        for (idx, contract) in cfg.begin_blockers.iter().enumerate() {
            debug!(idx, contract = contract.to_string(), "Calling begin blocker");
            // NOTE: error in begin blocker is considered fatal error. a begin
            // blocker erroring causes the chain to halt.
            // TODO: we need to think whether this is the desired behavior
            events.extend(before_block(cached.share(), &block, contract)?);
        }

        // process transactions one-by-one
        for (idx, raw_tx) in raw_txs.into_iter().enumerate() {
            debug!(idx, tx_hash = hash(raw_tx.as_ref()).to_string(), "Processing transaction");
            tx_results.push(run_tx(cached.share(), &block, raw_tx));
        }

        // call end blockers
        for (idx, contract) in cfg.end_blockers.iter().enumerate() {
            debug!(idx, contract = contract.to_string(), "Calling end blocker");
            // NOTE: error in end blocker is considered fatal error. an end
            // blocker erroring causes the chain to halt.
            // TODO: we need to think whether this is the desired behavior
            events.extend(after_block(cached.share(), &block, contract)?);
        }

        // save the last committed block
        //
        // note that we do this *after* the transactions have been executed, so
        // if a contract queries the last committed block during the execution,
        // it gets the previous block, not the current one.
        LAST_FINALIZED_BLOCK.save(&mut cached, &block)?;

        // flush the state changes to the DB, but keep it in memory, not persist
        // to disk yet. it will be done in the ABCI `Commit` call.
        let (_, batch) = cached.disassemble().disassemble();
        let (version, root_hash) = self.store.flush_but_not_commit(batch)?;

        // block height should match the DB version
        debug_assert_eq!(block.height.u64(), version);
        // the merkle tree should never be empty because at least we always have
        // things like the config, last finalized block, ...
        debug_assert!(root_hash.is_some());

        info!(
            height    = block.height.u64(),
            timestamp = block.timestamp.seconds(),
            app_hash  = root_hash.as_ref().unwrap().to_string(),
            "Finalized block"
        );

        Ok((root_hash.unwrap(), events, tx_results))
    }

    // TODO: we need to think about what to do if the flush fails here?
    pub fn do_commit(&self) -> AppResult<()> {
        self.store.commit()?;

        info!(height = self.store.latest_version(), "Committed state");

        Ok(())
    }

    // returns (last_block_height, last_block_app_hash)
    // note that we are returning the app hash, not the block hash
    pub fn do_info(&self) -> AppResult<(u64, Hash)> {
        let Some(version) = self.store.latest_version() else {
            // base store doesn't have a version. this is the case if the chain
            // hasn't started yet (prior to the InitChain call). in this case we
            // return zero height and an all-zero zero hash.
            return Ok((0, Hash::ZERO));
        };

        let Some(root_hash) = self.store.root_hash(Some(version))? else {
            // root hash is None. since we know version is not zero at this
            // point, the only way root hash is None is that state tree is empty.
            // however this is impossible, since we always keep some data in the
            // state (such as chain ID and config).
            panic!("root hash not found at the latest version ({version})");
        };

        Ok((version, root_hash))
    }

    pub fn do_query_app(&self, raw_query: &[u8], height: u64, prove: bool) -> AppResult<Binary> {
        if prove {
            // we can't do merkle proof for smart queries. only raw store query
            // can be merkle proved.
            return Err(AppError::ProofNotSupported);
        }

        let version = if height == 0 {
            // height being zero means unspecified (protobuf doesn't have a null
            // type) in which case we use the latest version.
            None
        } else {
            Some(height)
        };

        // use the state storage at the given version to perform the query
        let store = self.store.state_storage(version);
        let block = LAST_FINALIZED_BLOCK.load(&store)?;
        let req: QueryRequest = from_json(raw_query)?;
        let res = process_query(store, &block, req)?;

        Ok(to_json(&res)?)
    }

    pub fn do_query_store(
        &self,
        key:    &[u8],
        height: u64,
        prove:  bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Binary>)> {
        let version = if height == 0 {
            // height being zero means unspecified (protobuf doesn't have a null
            // type) in which case we use the latest version.
            None
        } else {
            Some(height)
        };

        let proof = if prove {
            Some(to_json(&self.store.prove(key, version)?)?)
        } else {
            None
        };

        let value = self.store.state_storage(version).read(key);

        Ok((value, proof))
    }
}

fn run_tx<S>(store: S, block: &BlockInfo, raw_tx: impl AsRef<[u8]>) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
{
    let tx = from_json(raw_tx)?;
    let mut events = vec![];

    // create cached store for this tx
    let cached = SharedStore::new(CacheStore::new(store, None));

    // call the sender account's `before_tx` method.
    // if this fails, abort, discard uncommitted state changes.
    events.extend(before_tx(cached.share(), block, &tx)?);

    // update the account state. as long as authentication succeeds, regardless
    // of whether the message are successful, we update account state. if auth
    // fails, we don't update account state.
    cached.write_access().commit();

    // now that the tx is authenticated, we loop through the messages and
    // execute them one by one.
    // if any one of the msgs fails, the entire tx fails; abort, discard
    // uncommitted changes (the changes from the before_tx call earlier are
    // persisted)
    for (idx, msg) in tx.msgs.iter().enumerate() {
        debug!(idx, "Processing message");
        events.extend(process_msg(cached.share(), block, &tx.sender, msg.clone())?);
    }

    // call the sender account's `after_tx` method.
    // if this fails, abort, discard uncommitted state changes from messages.
    // state changes from `before_tx` are always kept.
    events.extend(after_tx(cached.share(), block, &tx)?);

    // all messages succeeded. commit the state changes
    cached.write_access().commit();

    Ok(events)
}
