use {
    crate::{
        do_after_block, do_after_tx, do_before_block, do_before_tx, do_create_client, do_execute,
        do_freeze_client, do_instantiate, do_migrate, do_set_config, do_transfer, do_update_client,
        do_upload, query_account, query_accounts, query_balance, query_balances, query_code,
        query_codes, query_info, query_supplies, query_supply, query_wasm_raw, query_wasm_smart,
        AppError, AppResult, CacheStore, SharedStore, CHAIN_ID, CONFIG, LAST_FINALIZED_BLOCK,
    },
    cw_std::{
        from_json_slice, hash, to_json_vec, Addr, BlockInfo, Db, Event, GenesisState, Hash,
        Message, Permission, QueryRequest, QueryResponse, Storage, Vm, GENESIS_SENDER,
    },
    std::marker::PhantomData,
    tracing::{debug, info},
};

/// The ABCI application.
///
/// Must be clonable which is required by `tendermint-abci` library:
/// https://github.com/informalsystems/tendermint-rs/blob/v0.34.0/abci/src/application.rs#L22-L25
pub struct App<DB, VM> {
    db: DB,
    vm: PhantomData<VM>,
}

impl<DB, VM> App<DB, VM> {
    pub fn new(db: DB) -> Self {
        Self {
            db,
            vm: PhantomData,
        }
    }
}

// For some reason, using a derive macro `#[derive(Clone)]` on App doesn't work.
// The compiler demands that VM implements Clone before it will implement Clone
// for App; which doesn't make any sense because VM is just a PhantomData inside
// App...????
impl<DB, VM> Clone for App<DB, VM>
where
    DB: Clone,
{
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            vm: PhantomData,
        }
    }
}

impl<DB, VM> App<DB, VM>
where
    DB: Db,
    VM: Vm + 'static,
    AppError: From<DB::Error> + From<VM::Error>,
{
    pub fn do_init_chain(
        &self,
        chain_id: String,
        block: BlockInfo,
        app_state_bytes: &[u8],
    ) -> AppResult<Hash> {
        let mut cached = SharedStore::new(CacheStore::new(self.db.state_storage(None), None));

        // make sure the block height during InitChain is zero. this is necessary
        // to ensure that block height always matches the BaseStore version.
        if block.height.u64() != 0 {
            return Err(AppError::incorrect_block_height(0, block.height.u64()));
        }

        // deserialize the genesis state
        let genesis_state: GenesisState = from_json_slice(app_state_bytes)?;

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
            process_msg::<_, VM>(cached.clone(), &block, &GENESIS_SENDER, msg)?;
        }

        // persist the state changes to disk
        let (_, pending) = cached.disassemble().disassemble();
        let (version, root_hash) = self.db.flush_and_commit(pending)?;

        // BaseStore version should be 0
        debug_assert_eq!(version, 0);
        // the root hash should not be None. it's only None when the merkle tree
        // is empty, but we have written some data to it (like the chain ID and
        // the config) so it shouldn't be empty.
        debug_assert!(root_hash.is_some());

        info!(
            chain_id,
            timestamp = block.timestamp.seconds(),
            app_hash = root_hash.as_ref().unwrap().to_string(),
            "Completed genesis"
        );

        // return an empty apphash as placeholder, since we haven't implemented
        // state merklization yet
        Ok(root_hash.unwrap())
    }

    #[allow(clippy::type_complexity)]
    pub fn do_finalize_block(
        &self,
        block: BlockInfo,
        raw_txs: Vec<impl AsRef<[u8]>>,
    ) -> AppResult<(Hash, Vec<Event>, Vec<AppResult<Vec<Event>>>)> {
        let mut cached = SharedStore::new(CacheStore::new(self.db.state_storage(None), None));
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
            events.extend(do_before_block::<_, VM>(cached.share(), &block, contract)?);
        }

        // process transactions one-by-one
        for (idx, raw_tx) in raw_txs.into_iter().enumerate() {
            debug!(idx, tx_hash = hash(raw_tx.as_ref()).to_string(), "Processing transaction");
            tx_results.push(process_tx::<_, VM>(cached.share(), &block, raw_tx));
        }

        // call end blockers
        for (idx, contract) in cfg.end_blockers.iter().enumerate() {
            debug!(idx, contract = contract.to_string(), "Calling end blocker");
            // NOTE: error in end blocker is considered fatal error. an end
            // blocker erroring causes the chain to halt.
            // TODO: we need to think whether this is the desired behavior
            events.extend(do_after_block::<_, VM>(cached.share(), &block, contract)?);
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
        let (version, root_hash) = self.db.flush_but_not_commit(batch)?;

        // block height should match the DB version
        debug_assert_eq!(block.height.u64(), version);
        // the merkle tree should never be empty because at least we always have
        // things like the config, last finalized block, ...
        debug_assert!(root_hash.is_some());

        info!(
            height = block.height.u64(),
            timestamp = block.timestamp.seconds(),
            app_hash = root_hash.as_ref().unwrap().to_string(),
            "Finalized block"
        );

        Ok((root_hash.unwrap(), events, tx_results))
    }

    // TODO: we need to think about what to do if the flush fails here?
    pub fn do_commit(&self) -> AppResult<()> {
        self.db.commit()?;

        info!(height = self.db.latest_version(), "Committed state");

        Ok(())
    }

    // returns (last_block_height, last_block_app_hash)
    // note that we are returning the app hash, not the block hash
    pub fn do_info(&self) -> AppResult<(u64, Hash)> {
        let Some(version) = self.db.latest_version() else {
            // base store doesn't have a version. this is the case if the chain
            // hasn't started yet (prior to the InitChain call). in this case we
            // return zero height and an all-zero zero hash.
            return Ok((0, Hash::ZERO));
        };

        let Some(root_hash) = self.db.root_hash(Some(version))? else {
            // root hash is None. since we know version is not zero at this
            // point, the only way root hash is None is that state tree is empty.
            // however this is impossible, since we always keep some data in the
            // state (such as chain ID and config).
            panic!("root hash not found at the latest version ({version})");
        };

        Ok((version, root_hash))
    }

    pub fn do_query_app(&self, raw_query: &[u8], height: u64, prove: bool) -> AppResult<Vec<u8>> {
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
        let store = self.db.state_storage(version);
        let block = LAST_FINALIZED_BLOCK.load(&store)?;
        let req: QueryRequest = from_json_slice(raw_query)?;
        let res = process_query::<_, VM>(store, &block, req)?;

        Ok(to_json_vec(&res)?)
    }

    /// Performs a raw query of the app's underlying key-value store.
    /// Returns two values:
    /// - the value corresponding to the given key; `None` if the key doesn't exist;
    /// - the Merkle proof; `None` if a proof is not requested (`prove` is false).
    #[allow(clippy::type_complexity)]
    pub fn do_query_store(
        &self,
        key: &[u8],
        height: u64,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        let version = if height == 0 {
            // height being zero means unspecified (protobuf doesn't have a null
            // type) in which case we use the latest version.
            None
        } else {
            Some(height)
        };

        let proof = if prove {
            Some(to_json_vec(&self.db.prove(key, version)?)?)
        } else {
            None
        };

        let value = self.db.state_storage(version).read(key);

        Ok((value, proof))
    }
}

fn process_tx<S, VM>(store: S, block: &BlockInfo, raw_tx: impl AsRef<[u8]>) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let tx = from_json_slice(raw_tx)?;
    let mut events = vec![];

    // create cached store for this tx
    let cached = SharedStore::new(CacheStore::new(store, None));

    // call the sender account's `before_tx` method.
    // if this fails, abort, discard uncommitted state changes.
    events.extend(do_before_tx::<_, VM>(cached.share(), block, &tx)?);

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
        events.extend(process_msg::<_, VM>(cached.share(), block, &tx.sender, msg.clone())?);
    }

    // call the sender account's `after_tx` method.
    // if this fails, abort, discard uncommitted state changes from messages.
    // state changes from `before_tx` are always kept.
    events.extend(do_after_tx::<_, VM>(cached.share(), block, &tx)?);

    // all messages succeeded. commit the state changes
    cached.write_access().commit();

    Ok(events)
}

pub fn process_msg<S, VM>(
    mut store: S,
    block: &BlockInfo,
    sender: &Addr,
    msg: Message,
) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match msg {
        Message::SetConfig {
            new_cfg,
        } => do_set_config(&mut store, sender, &new_cfg),
        Message::Transfer {
            to,
            coins,
        } => do_transfer::<S, VM>(store, block, sender.clone(), to, coins, true),
        Message::Upload {
            code,
        } => do_upload(&mut store, sender, &code),
        Message::Instantiate {
            code_hash,
            msg,
            salt,
            funds,
            admin,
        } => do_instantiate::<S, VM>(store, block, sender, code_hash, &msg, salt, funds, admin),
        Message::Execute {
            contract,
            msg,
            funds,
        } => do_execute::<S, VM>(store, block, &contract, sender, &msg, funds),
        Message::Migrate {
            contract,
            new_code_hash,
            msg,
        } => do_migrate::<S, VM>(store, block, &contract, sender, new_code_hash, &msg),
        Message::CreateClient {
            code_hash,
            client_state,
            consensus_state,
            salt,
        } => do_create_client::<S, VM>(store, block, sender, code_hash, client_state, consensus_state, salt),
        Message::UpdateClient {
            client_id,
            header,
        } => do_update_client::<S, VM>(store, block, sender, &client_id, header),
        Message::FreezeClient {
            client_id,
            misbehavior,
        } => do_freeze_client::<S, VM>(store, block, sender, &client_id, misbehavior),
    }
}

pub fn process_query<S, VM>(
    store: S,
    block: &BlockInfo,
    req: QueryRequest,
) -> AppResult<QueryResponse>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match req {
        QueryRequest::Info {} => query_info(&store).map(QueryResponse::Info),
        QueryRequest::Balance {
            address,
            denom,
        } => query_balance::<S, VM>(store, block, address, denom).map(QueryResponse::Balance),
        QueryRequest::Balances {
            address,
            start_after,
            limit,
        } => query_balances::<S, VM>(store, block, address, start_after, limit).map(QueryResponse::Balances),
        QueryRequest::Supply {
            denom,
        } => query_supply::<S, VM>(store, block, denom).map(QueryResponse::Supply),
        QueryRequest::Supplies {
            start_after,
            limit,
        } => query_supplies::<S, VM>(store, block, start_after, limit).map(QueryResponse::Supplies),
        QueryRequest::Code {
            hash,
        } => query_code(&store, hash).map(QueryResponse::Code),
        QueryRequest::Codes {
            start_after,
            limit,
        } => query_codes(&store, start_after, limit).map(QueryResponse::Codes),
        QueryRequest::Account {
            address,
        } => query_account(&store, address).map(QueryResponse::Account),
        QueryRequest::Accounts {
            start_after,
            limit,
        } => query_accounts(&store, start_after, limit).map(QueryResponse::Accounts),
        QueryRequest::WasmRaw {
            contract,
            key,
        } => query_wasm_raw(store, contract, key).map(QueryResponse::WasmRaw),
        QueryRequest::WasmSmart {
            contract,
            msg,
        } => query_wasm_smart::<S, VM>(store, block, contract, msg).map(QueryResponse::WasmSmart),
    }
}

pub fn has_permission(permission: &Permission, owner: Option<&Addr>, sender: &Addr) -> bool {
    // the genesis sender can always store code and instantiate contracts
    if sender == GENESIS_SENDER {
        return true;
    }

    // owner can always do anything it wants
    if let Some(owner) = owner {
        if sender == owner {
            return true;
        }
    }

    match permission {
        Permission::Nobody => false,
        Permission::Everybody => true,
        Permission::Somebodies(accounts) => accounts.contains(sender),
    }
}
