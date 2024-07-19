use {
    crate::{
        do_after_tx, do_before_tx, do_configure, do_cron_execute, do_execute, do_instantiate,
        do_migrate, do_transfer, do_upload, query_account, query_accounts, query_balance,
        query_balances, query_code, query_codes, query_info, query_supplies, query_supply,
        query_wasm_raw, query_wasm_smart, AppError, AppResult, Buffer, Db, GasTracker, Shared, Vm,
        CHAIN_ID, CONFIG, LAST_FINALIZED_BLOCK, NEXT_CRONJOBS,
    },
    grug_storage::PrefixBound,
    grug_types::{
        from_json_slice, to_json_vec, Addr, BlockInfo, Duration, Event, GenesisState, Hash,
        Message, Order, Permission, QueryRequest, QueryResponse, StdResult, Storage, Timestamp, Tx,
        GENESIS_SENDER,
    },
};

/// The ABCI application.
///
/// Must be clonable which is required by `tendermint-abci` library:
/// <https://github.com/informalsystems/tendermint-rs/blob/v0.34.0/abci/src/application.rs#L22-L25>
#[derive(Clone)]
pub struct App<DB, VM> {
    db: DB,
    vm: VM,
    /// The gas limit when serving ABCI `Query` calls. `None` means no limit.
    ///
    /// Prevents the situation where an attacker deploys a contract that
    /// contains an extremely expensive query method (such as one containing an
    /// infinite loop), then makes a query request at a node. Without a gas
    /// limit, this can take down the node.
    ///
    /// Note that this is not relevant for queries made as part of a transaction,
    /// which is covered by the transaction's gas limit.
    ///
    /// Related config in CosmWasm:
    /// <https://github.com/CosmWasm/wasmd/blob/v0.51.0/x/wasm/types/types.go#L322-L323>
    query_gas_limit: Option<u64>,
}

impl<DB, VM> App<DB, VM> {
    pub fn new(db: DB, vm: VM, query_gas_limit: Option<u64>) -> Self {
        Self {
            db,
            vm,
            query_gas_limit,
        }
    }
}

impl<DB, VM> App<DB, VM>
where
    DB: Db,
    VM: Vm + Clone,
    AppError: From<DB::Error> + From<VM::Error>,
{
    pub fn do_init_chain_raw(
        &self,
        chain_id: String,
        block: BlockInfo,
        raw_genesis_state: &[u8],
    ) -> AppResult<Hash> {
        let genesis_state = from_json_slice(raw_genesis_state)?;
        self.do_init_chain(chain_id, block, genesis_state)
    }

    pub fn do_init_chain(
        &self,
        chain_id: String,
        block: BlockInfo,
        genesis_state: GenesisState,
    ) -> AppResult<Hash> {
        let mut buffer = Shared::new(Buffer::new(self.db.state_storage(None), None));

        // make sure the block height during InitChain is zero. this is necessary
        // to ensure that block height always matches the BaseStore version.
        if block.height.number() != 0 {
            return Err(AppError::IncorrectBlockHeight {
                expect: 0,
                actual: block.height.number(),
            });
        }

        // Create gas tracker for genesis.
        // During genesis, there is no gas limit.
        let gas_tracker = GasTracker::new_limitless();

        // save the config and genesis block. some genesis messages may need it
        CHAIN_ID.save(&mut buffer, &chain_id)?;
        CONFIG.save(&mut buffer, &genesis_state.config)?;
        LAST_FINALIZED_BLOCK.save(&mut buffer, &block)?;

        // Schedule cronjobs
        for (contract, interval) in genesis_state.config.cronjobs {
            schedule_cronjob(&mut buffer, &contract, block.timestamp, interval)?;
        }

        // loop through genesis messages and execute each one.
        // it's expected that genesis messages should all successfully execute.
        // if anyone fails, it's fatal error and we abort the genesis.
        // the developer should examine the error, fix it, and retry.
        for (_idx, msg) in genesis_state.msgs.into_iter().enumerate() {
            #[cfg(feature = "tracing")]
            tracing::info!(idx = _idx, "Processing genesis message");

            process_msg(
                self.vm.clone(),
                Box::new(buffer.clone()),
                gas_tracker.clone(),
                block.clone(),
                GENESIS_SENDER,
                msg,
            )?;
        }

        // persist the state changes to disk
        let (_, pending) = buffer.disassemble().disassemble();
        let (version, root_hash) = self.db.flush_and_commit(pending)?;

        // BaseStore version should be 0
        debug_assert_eq!(version, 0);

        // the root hash should not be None. it's only None when the merkle tree
        // is empty, but we have written some data to it (like the chain ID and
        // the config) so it shouldn't be empty.
        debug_assert!(root_hash.is_some());

        #[cfg(feature = "tracing")]
        tracing::info!(
            chain_id,
            time = into_utc_string(block.timestamp),
            app_hash = root_hash.as_ref().unwrap().to_string(),
            gas_used = gas_tracker.used(),
            "Completed genesis"
        );

        Ok(root_hash.unwrap())
    }

    pub fn do_finalize_block_raw<T>(
        &self,
        block: BlockInfo,
        raw_txs: &[T],
    ) -> AppResult<(Hash, Vec<Event>, Vec<AppResult<Vec<Event>>>)>
    where
        T: AsRef<[u8]>,
    {
        let txs = raw_txs
            .iter()
            .map(from_json_slice)
            .collect::<StdResult<Vec<_>>>()?;

        self.do_finalize_block(block, txs)
    }

    pub fn do_finalize_block(
        &self,
        block: BlockInfo,
        txs: Vec<Tx>,
    ) -> AppResult<(Hash, Vec<Event>, Vec<AppResult<Vec<Event>>>)> {
        let mut buffer = Shared::new(Buffer::new(self.db.state_storage(None), None));

        // Events emitted by cronjobs
        let mut cron_events = vec![];

        // Results of executing transactions
        let mut tx_results = vec![];

        let cfg = CONFIG.load(&buffer)?;
        let last_finalized_block = LAST_FINALIZED_BLOCK.load(&buffer)?;

        // make sure the new block height is exactly the last finalized height
        // plus one. this ensures that block height always matches the BaseStore
        // version.
        if block.height.number() != last_finalized_block.height.number() + 1 {
            return Err(AppError::IncorrectBlockHeight {
                expect: last_finalized_block.height.number() + 1,
                actual: block.height.number(),
            });
        }

        // Find all cronjobs that should be performed. That is, ones that the
        // scheduled time is earlier or equal to the current block time.
        let jobs = NEXT_CRONJOBS
            .prefix_range(
                &buffer,
                None,
                Some(PrefixBound::Inclusive(block.timestamp)),
                Order::Ascending,
            )
            .collect::<StdResult<Vec<_>>>()?;

        // Delete these cronjobs. They will be scheduled a new time.
        NEXT_CRONJOBS.prefix_clear(
            &mut buffer,
            None,
            Some(PrefixBound::Inclusive(block.timestamp)),
        );

        // Perform the cronjobs.
        for (_idx, (_time, contract)) in jobs.into_iter().enumerate() {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                idx = _idx,
                time = into_utc_string(_time),
                contract = contract.to_string(),
                "Attempting to perform cronjob"
            );

            if let Some(events) = do_cron_execute(
                self.vm.clone(),
                Box::new(buffer.clone()),
                GasTracker::new_limitless(),
                block.clone(),
                contract.clone(),
            ) {
                cron_events.extend(events);
            }

            // Schedule the next time this cronjob is to be performed.
            schedule_cronjob(
                &mut buffer,
                &contract,
                block.timestamp,
                cfg.cronjobs[&contract],
            )?;
        }

        // process transactions one-by-one
        for (_idx, tx) in txs.into_iter().enumerate() {
            #[cfg(feature = "tracing")]
            tracing::debug!(idx = _idx, "Processing transaction");

            tx_results.push(process_tx(
                self.vm.clone(),
                buffer.share(),
                block.clone(),
                tx,
            ));
        }

        // save the last committed block
        //
        // note that we do this *after* the transactions have been executed, so
        // if a contract queries the last committed block during the execution,
        // it gets the previous block, not the current one.
        LAST_FINALIZED_BLOCK.save(&mut buffer, &block)?;

        // flush the state changes to the DB, but keep it in memory, not persist
        // to disk yet. it will be done in the ABCI `Commit` call.
        let (_, batch) = buffer.disassemble().disassemble();
        let (version, root_hash) = self.db.flush_but_not_commit(batch)?;

        // block height should match the DB version
        debug_assert_eq!(block.height.number(), version);

        // the merkle tree should never be empty because at least we always have
        // things like the config, last finalized block, ...
        debug_assert!(root_hash.is_some());

        #[cfg(feature = "tracing")]
        tracing::info!(
            height = block.height.number(),
            time = into_utc_string(block.timestamp),
            app_hash = root_hash.as_ref().unwrap().to_string(),
            "Finalized block"
        );

        Ok((root_hash.unwrap(), cron_events, tx_results))
    }

    // TODO: we need to think about what to do if the flush fails here?
    pub fn do_commit(&self) -> AppResult<()> {
        self.db.commit()?;

        #[cfg(feature = "tracing")]
        tracing::info!(height = self.db.latest_version(), "Committed state");

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

    pub fn do_query_app_raw(&self, raw_req: &[u8], height: u64, prove: bool) -> AppResult<Vec<u8>> {
        let req = from_json_slice(raw_req)?;
        let res = self.do_query_app(req, height, prove)?;
        Ok(to_json_vec(&res)?)
    }

    pub fn do_query_app(
        &self,
        req: QueryRequest,
        height: u64,
        prove: bool,
    ) -> AppResult<QueryResponse> {
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

        process_query(
            self.vm.clone(),
            Box::new(store),
            GasTracker::new(self.query_gas_limit),
            block,
            req,
        )
    }

    /// Performs a raw query of the app's underlying key-value store.
    /// Returns two values:
    /// - the value corresponding to the given key; `None` if the key doesn't exist;
    /// - the Merkle proof; `None` if a proof is not requested (`prove` is false).
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

fn process_tx<S, VM>(vm: VM, storage: S, block: BlockInfo, tx: Tx) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut events = vec![];

    // create buffer storage and gas tracker for this tx
    let buffer = Shared::new(Buffer::new(storage, None));
    let gas_tracker = GasTracker::new_limited(tx.gas_limit);

    // call the sender account's `before_tx` method.
    // if this fails, abort, discard uncommitted state changes.
    events.extend(do_before_tx(
        vm.clone(),
        Box::new(buffer.share()),
        gas_tracker.clone(),
        block.clone(),
        &tx,
    )?);

    // update the account state. as long as authentication succeeds, regardless
    // of whether the message are successful, we update account state. if auth
    // fails, we don't update account state.
    buffer.write_access().commit();

    // now that the tx is authenticated, we loop through the messages and
    // execute them one by one.
    // if any one of the msgs fails, the entire tx fails; abort, discard
    // uncommitted changes (the changes from the before_tx call earlier are
    // persisted)
    for (_idx, msg) in tx.msgs.iter().enumerate() {
        #[cfg(feature = "tracing")]
        tracing::debug!(idx = _idx, "Processing message");

        events.extend(process_msg(
            vm.clone(),
            Box::new(buffer.share()),
            gas_tracker.clone(),
            block.clone(),
            tx.sender.clone(),
            msg.clone(),
        )?);
    }

    // call the sender account's `after_tx` method.
    // if this fails, abort, discard uncommitted state changes from messages.
    // state changes from `before_tx` are always kept.
    events.extend(do_after_tx(
        vm,
        Box::new(buffer.share()),
        gas_tracker.clone(),
        block,
        &tx,
    )?);

    // all messages succeeded. commit the state changes
    buffer.write_access().commit();

    Ok(events)
}

pub fn process_msg<VM>(
    vm: VM,
    mut storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    sender: Addr,
    msg: Message,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match msg {
        Message::Configure { new_cfg } => do_configure(&mut storage, block, &sender, new_cfg),
        Message::Transfer { to, coins } => do_transfer(
            vm,
            storage,
            gas_tracker,
            block,
            sender.clone(),
            to,
            coins,
            true,
        ),
        Message::Upload { code } => do_upload(&mut storage, &sender, code.into()),
        Message::Instantiate {
            code_hash,
            msg,
            salt,
            funds,
            admin,
        } => do_instantiate(
            vm,
            storage,
            gas_tracker,
            block,
            sender,
            code_hash,
            &msg,
            salt,
            funds,
            admin,
        ),
        Message::Execute {
            contract,
            msg,
            funds,
        } => do_execute(
            vm,
            storage,
            gas_tracker,
            block,
            contract,
            sender,
            &msg,
            funds,
        ),
        Message::Migrate {
            contract,
            new_code_hash,
            msg,
        } => do_migrate(
            vm,
            storage,
            gas_tracker,
            block,
            contract,
            sender,
            new_code_hash,
            &msg,
        ),
    }
}

pub fn process_query<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    req: QueryRequest,
) -> AppResult<QueryResponse>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match req {
        QueryRequest::Info {} => query_info(&storage).map(QueryResponse::Info),
        QueryRequest::Balance { address, denom } => {
            query_balance(vm, storage, block, gas_tracker, address, denom)
                .map(QueryResponse::Balance)
        },
        QueryRequest::Balances {
            address,
            start_after,
            limit,
        } => query_balances(vm, storage, block, gas_tracker, address, start_after, limit)
            .map(QueryResponse::Balances),
        QueryRequest::Supply { denom } => {
            query_supply(vm, storage, block, gas_tracker, denom).map(QueryResponse::Supply)
        },
        QueryRequest::Supplies { start_after, limit } => {
            query_supplies(vm, storage, block, gas_tracker, start_after, limit)
                .map(QueryResponse::Supplies)
        },
        QueryRequest::Code { hash } => query_code(&storage, hash).map(QueryResponse::Code),
        QueryRequest::Codes { start_after, limit } => {
            query_codes(&storage, start_after, limit).map(QueryResponse::Codes)
        },
        QueryRequest::Account { address } => {
            query_account(&storage, address).map(QueryResponse::Account)
        },
        QueryRequest::Accounts { start_after, limit } => {
            query_accounts(&storage, start_after, limit).map(QueryResponse::Accounts)
        },
        QueryRequest::WasmRaw { contract, key } => {
            query_wasm_raw(storage, contract, key).map(QueryResponse::WasmRaw)
        },
        QueryRequest::WasmSmart { contract, msg } => {
            query_wasm_smart(vm, storage, block, gas_tracker, contract, msg)
                .map(QueryResponse::WasmSmart)
        },
    }
}

pub(crate) fn has_permission(permission: &Permission, owner: Option<&Addr>, sender: &Addr) -> bool {
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

pub(crate) fn schedule_cronjob(
    storage: &mut dyn Storage,
    contract: &Addr,
    current_time: Timestamp,
    interval: Duration,
) -> StdResult<()> {
    let next_time = current_time + interval;

    #[cfg(feature = "tracing")]
    tracing::info!(
        time = into_utc_string(next_time),
        contract = contract.to_string(),
        "Scheduled cronjob"
    );

    NEXT_CRONJOBS.insert(storage, (next_time, contract))
}

#[cfg(feature = "tracing")]
pub fn into_utc_string(timestamp: Timestamp) -> String {
    // This panics if the timestamp (as nanoseconds) overflows `i64` range.
    // But that'd be 500 years or so from now...
    chrono::DateTime::from_timestamp_nanos(timestamp.into_nanos() as i64)
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}
