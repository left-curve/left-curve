use crate::{query_app_config, query_app_configs, APP_CONFIGS};
#[cfg(feature = "abci")]
use grug_types::from_json_slice;

use {
    crate::{
        do_authenticate, do_backrun, do_configure, do_cron_execute, do_execute, do_finalize_fee,
        do_instantiate, do_migrate, do_transfer, do_upload, do_withhold_fee, query_account,
        query_accounts, query_balance, query_balances, query_code, query_codes, query_info,
        query_supplies, query_supply, query_wasm_raw, query_wasm_smart, AppError, AppResult,
        Buffer, Db, GasTracker, Shared, Vm, CHAIN_ID, CONFIG, LAST_FINALIZED_BLOCK, NEXT_CRONJOBS,
    },
    grug_storage::PrefixBound,
    grug_types::{
        to_json_vec, Addr, AuthMode, BlockInfo, BlockOutcome, Duration, Event, GenesisState,
        Hash256, Json, Message, Order, Outcome, Permission, Query, QueryResponse, StdResult,
        Storage, Timestamp, Tx, TxOutcome, UnsignedTx, GENESIS_SENDER,
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
    /// The gas limit when serving ABCI `Query` calls.
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
    query_gas_limit: u64,
}

impl<DB, VM> App<DB, VM> {
    pub fn new(db: DB, vm: VM, query_gas_limit: u64) -> Self {
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
    pub fn do_init_chain(
        &self,
        chain_id: String,
        block: BlockInfo,
        genesis_state: GenesisState,
    ) -> AppResult<Hash256> {
        let mut buffer = Shared::new(Buffer::new(self.db.state_storage(None)?, None));

        // Make sure the genesis block height is zero. This is necessary to
        // ensure that block height always matches the DB version.
        if block.height.number() != 0 {
            return Err(AppError::IncorrectBlockHeight {
                expect: 0,
                actual: block.height.number(),
            });
        }

        // Create gas tracker for genesis.
        // During genesis, there is no gas limit.
        let gas_tracker = GasTracker::new_limitless();

        // Save the config and genesis block, so that they can be queried when
        // executing genesis messages.
        CHAIN_ID.save(&mut buffer, &chain_id)?;
        CONFIG.save(&mut buffer, &genesis_state.config)?;
        LAST_FINALIZED_BLOCK.save(&mut buffer, &block)?;

        // Save app configs.
        for (key, value) in genesis_state.app_configs {
            APP_CONFIGS.save(&mut buffer, &key, &value)?;
        }

        // Schedule cronjobs.
        for (contract, interval) in genesis_state.config.cronjobs {
            schedule_cronjob(&mut buffer, contract, block.timestamp, interval)?;
        }

        // Loop through genesis messages and execute each one.
        //
        // It's expected that genesis messages should all successfully execute.
        // If anyone fails, it's considered fatal and genesis is aborted.
        // The developer should examine the error, fix it, and retry.
        for (_idx, msg) in genesis_state.msgs.into_iter().enumerate() {
            #[cfg(feature = "tracing")]
            tracing::info!(idx = _idx, "Processing genesis message");

            process_msg(
                self.vm.clone(),
                Box::new(buffer.clone()),
                gas_tracker.clone(),
                block,
                GENESIS_SENDER,
                msg,
            )?;
        }

        // Persist the state changes to disk
        let (_, pending) = buffer.disassemble().disassemble();
        let (version, root_hash) = self.db.flush_and_commit(pending)?;

        // Sanity check: DB version should be 0
        debug_assert_eq!(version, 0);

        // Sanity check: the root hash should not be `None`.
        //
        // It's only `None` when the Merkle tree is empty, but we have written
        // some data to it (like chain ID and config) so it shouldn't be empty.
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

    pub fn do_finalize_block(&self, block: BlockInfo, txs: Vec<Tx>) -> AppResult<BlockOutcome> {
        let mut buffer = Shared::new(Buffer::new(self.db.state_storage(None)?, None));

        let mut cron_outcomes = vec![];
        let mut tx_outcomes = vec![];

        let cfg = CONFIG.load(&buffer)?;
        let last_finalized_block = LAST_FINALIZED_BLOCK.load(&buffer)?;

        // Make sure the new block height is exactly the last finalized height
        // plus one. This ensures that block height always matches the DB version.
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

            // Cronjobs can use unlimited gas
            let gas_tracker = GasTracker::new_limitless();

            let result = do_cron_execute(
                self.vm.clone(),
                Box::new(buffer.clone()),
                gas_tracker.clone(),
                block,
                contract,
            );

            cron_outcomes.push(new_outcome(gas_tracker, result));

            // Schedule the next time this cronjob is to be performed.
            schedule_cronjob(
                &mut buffer,
                contract,
                block.timestamp,
                cfg.cronjobs[&contract],
            )?;
        }

        // Process transactions one-by-one.
        for (_idx, tx) in txs.into_iter().enumerate() {
            #[cfg(feature = "tracing")]
            tracing::debug!(idx = _idx, "Processing transaction");

            tx_outcomes.push(process_tx(
                self.vm.clone(),
                buffer.clone(),
                block,
                tx,
                AuthMode::Finalize,
            ));
        }

        // Save the last committed block.
        //
        // Note that we do this _after_ the transactions have been executed.
        // If a contract queries the last committed block during the execution,
        // it gets the previous block, not the current one.
        LAST_FINALIZED_BLOCK.save(&mut buffer, &block)?;

        // Flush the state changes to the DB, but keep it in memory, not persist
        // to disk yet. It will be done in the ABCI `Commit` call.
        let (_, batch) = buffer.disassemble().disassemble();
        let (version, app_hash) = self.db.flush_but_not_commit(batch)?;

        // Sanity checks, same as in `do_init_chain`:
        // - Block height matches DB version
        // - Merkle tree isn't empty
        debug_assert_eq!(block.height.number(), version);
        debug_assert!(app_hash.is_some());

        #[cfg(feature = "tracing")]
        tracing::info!(
            height = block.height.number(),
            time = into_utc_string(block.timestamp),
            app_hash = app_hash.as_ref().unwrap().to_string(),
            "Finalized block"
        );

        Ok(BlockOutcome {
            app_hash: app_hash.unwrap(),
            cron_outcomes,
            tx_outcomes,
        })
    }

    pub fn do_commit(&self) -> AppResult<()> {
        self.db.commit()?;

        #[cfg(feature = "tracing")]
        tracing::info!(height = self.db.latest_version(), "Committed state");

        Ok(())
    }

    // For `CheckTx`, we only do the first two steps of the transaction
    // processing flow:
    // 1.`withhold_fee`, where the taxman makes sure the sender has sufficient
    //   tokens to cover the tx fee;
    // 2. `authenticate`, where the sender account authenticates the transaction.
    pub fn do_check_tx(&self, tx: Tx) -> AppResult<Outcome> {
        let buffer = Shared::new(Buffer::new(self.db.state_storage(None)?, None));
        let block = LAST_FINALIZED_BLOCK.load(&buffer)?;
        let gas_tracker = GasTracker::new_limited(tx.gas_limit);
        let mut events = vec![];

        match do_withhold_fee(
            self.vm.clone(),
            Box::new(buffer.clone()),
            GasTracker::new_limitless(),
            block,
            &tx,
        ) {
            Ok(new_events) => {
                events.extend(new_events);
            },
            Err(err) => {
                return Ok(new_outcome(gas_tracker, Err(err)));
            },
        }

        match do_authenticate(
            self.vm.clone(),
            Box::new(buffer),
            gas_tracker.clone(),
            block,
            &tx,
            AuthMode::Check,
        ) {
            Ok((new_events, _)) => {
                events.extend(new_events);
            },
            Err(err) => {
                return Ok(new_outcome(gas_tracker, Err(err)));
            },
        }

        Ok(new_outcome(gas_tracker, Ok(events)))
    }

    // Returns (last_block_height, last_block_app_hash).
    // Note that we are returning the app hash, not the block hash.
    pub fn do_info(&self) -> AppResult<(u64, Hash256)> {
        let Some(version) = self.db.latest_version() else {
            // The DB doesn't have a version yet. This is the case if the chain
            // hasn't started yet (prior to the `InitChain` call). In this case,
            // we return zero height and an all-zero zero hash.
            return Ok((0, Hash256::ZERO));
        };

        let Some(root_hash) = self.db.root_hash(Some(version))? else {
            // Root hash is `None`. Since we know version is not zero at this
            // point, the only way root hash is `None` is that state tree is
            // empty. However this is impossible, since we always keep some data
            // in the state (such as chain ID and config).
            panic!("root hash not found at the latest version ({version})");
        };

        Ok((version, root_hash))
    }

    pub fn do_query_app(&self, req: Query, height: u64, prove: bool) -> AppResult<QueryResponse> {
        if prove {
            // We can't do Merkle proof for smart queries. Only raw store query
            // can be Merkle proved.
            return Err(AppError::ProofNotSupported);
        }

        let version = if height == 0 {
            // Height being zero means unspecified (Protobuf doesn't have a null
            // type) in which case we use the latest version.
            None
        } else {
            Some(height)
        };

        // Use the state storage at the given version to perform the query.
        let storage = self.db.state_storage(version)?;
        let block = LAST_FINALIZED_BLOCK.load(&storage)?;

        // The gas limit for serving this query.
        // This is set as an off-chain, per-node parameter.
        let gas_tracker = GasTracker::new_limited(self.query_gas_limit);

        process_query(self.vm.clone(), Box::new(storage), gas_tracker, block, req)
    }

    /// Performs a raw query of the app's underlying key-value store.
    ///
    /// Returns:
    /// - the value corresponding to the given key; `None` if the key doesn't exist;
    /// - the Merkle proof; `None` if a proof is not requested (`prove` is false).
    pub fn do_query_store(
        &self,
        key: &[u8],
        height: u64,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        let version = if height == 0 {
            // Height being zero means unspecified (Protobuf doesn't have a null
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

        let value = self.db.state_storage(version)?.read(key);

        Ok((value, proof))
    }

    pub fn do_simulate(
        &self,
        unsigned_tx: UnsignedTx,
        height: u64,
        prove: bool,
    ) -> AppResult<TxOutcome> {
        let buffer = Buffer::new(self.db.state_storage(None)?, None);

        let block = LAST_FINALIZED_BLOCK.load(&buffer)?;

        // We can't "prove" a gas simulation
        if prove {
            return Err(AppError::ProofNotSupported);
        }

        // We can't simulate gas at a block height
        if height != 0 && height != block.height.number() {
            return Err(AppError::PastHeightNotSupported);
        }

        // Create a `Tx` from the unsigned transaction.
        // Use using the node's query gas limit as the transaction gas limit,
        // and empty bytes as credential.
        let tx = Tx {
            sender: unsigned_tx.sender,
            gas_limit: self.query_gas_limit,
            msgs: unsigned_tx.msgs,
            data: Json::Null,
            credential: Json::Null,
        };

        // Run the transaction with `simulate` as `true`. Track how much gas was
        // consumed, and, if it was successful, what events were emitted.
        Ok(process_tx(
            self.vm.clone(),
            buffer,
            block,
            tx,
            AuthMode::Simulate,
        ))
    }
}

#[cfg(feature = "abci")]
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
    ) -> AppResult<Hash256> {
        let genesis_state = from_json_slice(raw_genesis_state)?;

        self.do_init_chain(chain_id, block, genesis_state)
    }

    pub fn do_finalize_block_raw<T>(
        &self,
        block: BlockInfo,
        raw_txs: &[T],
    ) -> AppResult<BlockOutcome>
    where
        T: AsRef<[u8]>,
    {
        let txs = raw_txs
            .iter()
            .map(from_json_slice)
            .collect::<StdResult<Vec<_>>>()?;

        self.do_finalize_block(block, txs)
    }

    pub fn do_check_tx_raw(&self, raw_tx: &[u8]) -> AppResult<Outcome> {
        let tx = from_json_slice(raw_tx)?;

        self.do_check_tx(tx)
    }

    pub fn do_simulate_raw(
        &self,
        raw_unsigned_tx: &[u8],
        height: u64,
        prove: bool,
    ) -> AppResult<Vec<u8>> {
        let tx = from_json_slice(raw_unsigned_tx)?;
        let res = self.do_simulate(tx, height, prove)?;

        Ok(to_json_vec(&res)?)
    }

    pub fn do_query_app_raw(&self, raw_req: &[u8], height: u64, prove: bool) -> AppResult<Vec<u8>> {
        let req = from_json_slice(raw_req)?;
        let res = self.do_query_app(req, height, prove)?;

        Ok(to_json_vec(&res)?)
    }
}

fn process_tx<S, VM>(vm: VM, storage: S, block: BlockInfo, tx: Tx, mode: AuthMode) -> TxOutcome
where
    S: Storage + Clone + 'static,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Create two layers of buffers.
    //
    // The 1st layer (`buffer1`) is for fee handling; the 2nd layer (`buffer2`)
    // is for tx authentication and processing of the messages.
    let buffer1 = Shared::new(Buffer::new(storage, None));
    let buffer2 = Shared::new(Buffer::new(buffer1.clone(), None));

    // Create the gas tracker, with the limit being the gas limit requested by
    // the transaction.
    let gas_tracker = GasTracker::new_limited(tx.gas_limit);

    // Record the events emitted during the processing of this transaction.
    let mut events = Vec::new();

    // Call the taxman's `withhold_fee` function.
    //
    // The purpose of this step is to ensure the tx's sender has sufficient
    // token balance to cover the maximum possible fee the tx may incur.
    //
    // If this succeeds, record the events emitted.
    //
    // If this fails, we abort the tx and return, discard all state changes.
    match do_withhold_fee(
        vm.clone(),
        Box::new(buffer1.clone()),
        GasTracker::new_limitless(),
        block,
        &tx,
    ) {
        Ok(new_events) => {
            events.extend(new_events);
        },
        Err(err) => {
            return new_tx_outcome(gas_tracker.clone(), events.clone(), Err(err));
        },
    }

    // Call the sender account's `authenticate` function.
    //
    // The sender account is supposed to perform authentication here, such as
    // verifying a cryptographic signature, to ensure the tx comes from the
    // sender account's rightful owner.
    //
    // Note that we use `buffer2` for this.
    //
    // If succeeds, commit state changes in `buffer2` into `buffer1`, and record
    // the events emitted.
    //
    // If fails, discard state changes in `buffer2` (but keeping those in
    // `buffer1`), discard the events, and jump to `finalize_fee`.
    let request_backrun = match do_authenticate(
        vm.clone(),
        Box::new(buffer2.clone()),
        gas_tracker.clone(),
        block,
        &tx,
        mode,
    ) {
        Ok((new_events, request_backrun)) => {
            buffer2.write_access().commit();
            events.extend(new_events);
            request_backrun
        },
        Err(err) => {
            drop(buffer2);
            return process_finalize_fee(vm, buffer1, gas_tracker, block, tx, events, Err(err));
        },
    };

    // Loop through the messages and execute one by one. Then, call the sender
    // account's `backrun` method.
    //
    // If everything succeeds, commit state changes in `buffer2` into `buffer1`,
    // and record the events emitted.
    //
    // If anything fails, discard state changes in `buffer2` (but keeping those
    // in `buffer1`), discard the events, and jump to `finalize_fee`.
    match process_msgs_then_backrun(
        vm.clone(),
        buffer2.clone(),
        gas_tracker.clone(),
        block,
        &tx,
        mode,
        request_backrun,
    ) {
        Ok(new_events) => {
            buffer2.disassemble().consume();
            events.extend(new_events);
        },
        Err(err) => {
            drop(buffer2);
            return process_finalize_fee(vm, buffer1, gas_tracker, block, tx, events, Err(err));
        },
    }

    // Everything so far succeeded. Finally, call the taxman's `finalize_fee`
    // function.
    //
    // If the transaction didn't use up all the gas it has requested, it can get
    // a refund here (if taxman is programmed to do so).
    //
    // Taxman should be designed such that this call always succeeds. This
    // failing can be considered an "undefined behavior". In such a case, we
    // discard all previous state changes and events, as if the tx never happened.
    // Also, print a tracing message at the ERROR level to the CLI, to raise
    // developer's awareness.
    process_finalize_fee(vm, buffer1, gas_tracker, block, tx, events, Ok(()))
}

#[inline]
fn process_msgs_then_backrun<S, VM>(
    vm: VM,
    buffer: Shared<Buffer<S>>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
    request_backrun: bool,
) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut msg_events = Vec::new();

    for (_idx, msg) in tx.msgs.iter().enumerate() {
        #[cfg(feature = "tracing")]
        tracing::debug!(idx = _idx, "Processing message");

        msg_events.extend(process_msg(
            vm.clone(),
            Box::new(buffer.clone()),
            gas_tracker.clone(),
            block,
            tx.sender,
            msg.clone(),
        )?);
    }

    if request_backrun {
        msg_events.extend(do_backrun(
            vm.clone(),
            Box::new(buffer),
            gas_tracker,
            block,
            tx,
            mode,
        )?);
    }

    Ok(msg_events)
}

fn process_finalize_fee<S, VM>(
    vm: VM,
    buffer: Shared<Buffer<S>>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: Tx,
    mut events: Vec<Event>,
    result: AppResult<()>,
) -> TxOutcome
where
    S: Storage + Clone + 'static,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let outcome_so_far = new_tx_outcome(gas_tracker.clone(), events.clone(), result.clone());

    match do_finalize_fee(
        vm,
        Box::new(buffer.clone()),
        GasTracker::new_limitless(),
        block,
        &tx,
        &outcome_so_far,
    ) {
        Ok(new_events) => {
            events.extend(new_events);
            buffer.disassemble().consume();
            new_tx_outcome(gas_tracker, events, result)
        },
        Err(err) => {
            events.clear();
            drop(buffer);
            new_tx_outcome(gas_tracker, Vec::new(), Err(err))
        },
    }
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
        Message::Configure {
            updates,
            app_updates,
        } => do_configure(&mut storage, block, sender, updates, app_updates),
        Message::Transfer { to, coins } => {
            do_transfer(vm, storage, gas_tracker, block, sender, to, coins, true)
        },
        Message::Upload { code } => do_upload(&mut storage, gas_tracker, sender, &code),
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
    req: Query,
) -> AppResult<QueryResponse>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match req {
        Query::Info {} => {
            let res = query_info(&storage, gas_tracker)?;
            Ok(QueryResponse::Info(res))
        },
        Query::AppConfig { key } => {
            let res = query_app_config(&storage, gas_tracker, &key)?;
            Ok(QueryResponse::AppConfig(res))
        },
        Query::AppConfigs { start_after, limit } => {
            let res = query_app_configs(&storage, gas_tracker, start_after, limit)?;
            Ok(QueryResponse::AppConfigs(res))
        },
        Query::Balance { address, denom } => {
            let res = query_balance(vm, storage, block, gas_tracker, address, denom)?;
            Ok(QueryResponse::Balance(res))
        },
        Query::Balances {
            address,
            start_after,
            limit,
        } => {
            let res = query_balances(vm, storage, block, gas_tracker, address, start_after, limit)?;
            Ok(QueryResponse::Balances(res))
        },
        Query::Supply { denom } => {
            let res = query_supply(vm, storage, block, gas_tracker, denom)?;
            Ok(QueryResponse::Supply(res))
        },
        Query::Supplies { start_after, limit } => {
            let res = query_supplies(vm, storage, block, gas_tracker, start_after, limit)?;
            Ok(QueryResponse::Supplies(res))
        },
        Query::Code { hash } => {
            let res = query_code(&storage, gas_tracker, hash)?;
            Ok(QueryResponse::Code(res))
        },
        Query::Codes { start_after, limit } => {
            let res = query_codes(&storage, gas_tracker, start_after, limit)?;
            Ok(QueryResponse::Codes(res))
        },
        Query::Account { address } => {
            let res = query_account(&storage, gas_tracker, address)?;
            Ok(QueryResponse::Account(res))
        },
        Query::Accounts { start_after, limit } => {
            let res = query_accounts(&storage, gas_tracker, start_after, limit)?;
            Ok(QueryResponse::Accounts(res))
        },
        Query::WasmRaw { contract, key } => {
            let res = query_wasm_raw(storage, gas_tracker, contract, key)?;
            Ok(QueryResponse::WasmRaw(res))
        },
        Query::WasmSmart { contract, msg } => {
            let res = query_wasm_smart(vm, storage, block, gas_tracker, contract, msg)?;
            Ok(QueryResponse::WasmSmart(res))
        },
        Query::Multi(reqs) => {
            let res = reqs
                .into_iter()
                .map(|req| {
                    process_query(vm.clone(), storage.clone(), gas_tracker.clone(), block, req)
                })
                .collect::<AppResult<Vec<_>>>()?;
            Ok(QueryResponse::Multi(res))
        },
    }
}

pub(crate) fn has_permission(permission: &Permission, owner: Addr, sender: Addr) -> bool {
    // The genesis sender can always store code and instantiate contracts.
    if sender == GENESIS_SENDER {
        return true;
    }

    // The owner can always do anything it wants.
    if sender == owner {
        return true;
    }

    match permission {
        Permission::Nobody => false,
        Permission::Everybody => true,
        Permission::Somebodies(accounts) => accounts.contains(&sender),
    }
}

pub(crate) fn schedule_cronjob(
    storage: &mut dyn Storage,
    contract: Addr,
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

fn new_outcome(gas_tracker: GasTracker, result: AppResult<Vec<Event>>) -> Outcome {
    Outcome {
        gas_limit: gas_tracker.limit(),
        gas_used: gas_tracker.used(),
        result: result.into(),
    }
}

fn new_tx_outcome(gas_tracker: GasTracker, events: Vec<Event>, result: AppResult<()>) -> TxOutcome {
    TxOutcome {
        gas_limit: gas_tracker.limit().unwrap(),
        gas_used: gas_tracker.used(),
        events,
        result: result.into(),
    }
}

#[cfg(feature = "tracing")]
pub fn into_utc_string(timestamp: Timestamp) -> String {
    // This panics if the timestamp (as nanoseconds) overflows `i64` range.
    // But that'd be 500 years or so from now...
    chrono::DateTime::from_timestamp_nanos(timestamp.into_nanos() as i64)
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}
