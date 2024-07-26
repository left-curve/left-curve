#[cfg(feature = "abci")]
use grug_types::from_json_slice;
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
        to_json_vec, Addr, Binary, BlockInfo, Duration, Event, GenericResult, GenesisState, Hash,
        Message, Order, Permission, QueryRequest, QueryResponse, StdResult, Storage, Timestamp, Tx,
        UnsignedTx, GENESIS_SENDER,
    },
    serde::{Deserialize, Serialize},
};

/// Outcome of executing a block.
pub struct BlockOutcome {
    /// The Merkle root hash after executing this block.
    pub app_hash: Hash,
    /// Results of executing the cronjobs.
    pub cron_outcomes: Vec<Outcome>,
    /// Results of executing the transactions.
    pub tx_outcomes: Vec<Outcome>,
}

/// Outcome of executing a single message, transaction, or cronjob.
///
/// Includes the events emitted, and gas consumption.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    // `None` means the call was done with unlimited gas, such as cronjobs.
    pub gas_limit: Option<u64>,
    pub gas_used: u64,
    pub result: GenericResult<Vec<Event>>,
}

impl Outcome {
    pub fn new(gas_tracker: GasTracker, result: AppResult<Vec<Event>>) -> Self {
        Self {
            gas_limit: gas_tracker.limit(),
            gas_used: gas_tracker.used(),
            result: result.into(),
        }
    }
}

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
    ) -> AppResult<Hash> {
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

        // Schedule cronjobs.
        for (contract, interval) in genesis_state.config.cronjobs {
            schedule_cronjob(&mut buffer, &contract, block.timestamp, interval)?;
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
                block.clone(),
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
                block.clone(),
                contract.clone(),
            );

            cron_outcomes.push(Outcome::new(gas_tracker, result));

            // Schedule the next time this cronjob is to be performed.
            schedule_cronjob(
                &mut buffer,
                &contract,
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
                block.clone(),
                tx,
                false,
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

    // For `CheckTx`, we only do the `before_tx` part of the transaction, which
    // is where the sender account is supposed to do authentication.
    pub fn do_check_tx(&self, tx: Tx) -> AppResult<Outcome> {
        let buffer = Shared::new(Buffer::new(self.db.state_storage(None)?, None));
        let block = LAST_FINALIZED_BLOCK.load(&buffer)?;
        let gas_tracker = GasTracker::new_limited(tx.gas_limit);

        let result = do_before_tx(
            self.vm.clone(),
            Box::new(buffer),
            gas_tracker.clone(),
            block,
            &tx,
            false,
        );

        Ok(Outcome::new(gas_tracker, result))
    }

    // Returns (last_block_height, last_block_app_hash).
    // Note that we are returning the app hash, not the block hash.
    pub fn do_info(&self) -> AppResult<(u64, Hash)> {
        let Some(version) = self.db.latest_version() else {
            // The DB doesn't have a version yet. This is the case if the chain
            // hasn't started yet (prior to the `InitChain` call). In this case,
            // we return zero height and an all-zero zero hash.
            return Ok((0, Hash::ZERO));
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

    pub fn do_query_app(
        &self,
        req: QueryRequest,
        height: u64,
        prove: bool,
    ) -> AppResult<QueryResponse> {
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
    ) -> AppResult<Outcome> {
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
            credential: Binary::empty(),
        };

        // Run the transaction with `simulate` as `true`. Track how much gas was
        // consumed, and, if it was successful, what events were emitted.
        Ok(process_tx(self.vm.clone(), buffer, block, tx, true))
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
    ) -> AppResult<Hash> {
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

fn process_tx<S, VM>(vm: VM, storage: S, block: BlockInfo, tx: Tx, simulate: bool) -> Outcome
where
    S: Storage + Clone + 'static,
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let buffer = Shared::new(Buffer::new(storage, None));
    let gas_tracker = GasTracker::new_limited(tx.gas_limit);
    let mut events = vec![];

    macro_rules! try_do {
        ($call:expr) => {
            match $call {
                Ok(call_events) => events.extend(call_events),
                Err(err) => {
                    return Outcome::new(gas_tracker, Err(err));
                },
            }
        };
    }

    // Call the sender account's `before_tx` method.
    //
    // The account is expected to perform authentication at this time, such as
    // verifying a cryptographic signature.
    //
    // If this fails, abort, and discard uncommitted state changes.
    try_do! {
        do_before_tx(
            vm.clone(),
            Box::new(buffer.clone()),
            gas_tracker.clone(),
            block.clone(),
            &tx,
            simulate,
        )
    };

    // Update the account state. As long as authentication succeeds, regardless
    // of whether the message are successful, we update account state.
    //
    // The account may maintain a sequence number, for example, which needs to
    // be incremented even if the transaction fails.
    buffer.write_access().commit();

    // Now that the tx is authenticated, we loop through the messages and
    // execute them one by one.
    //
    // If any one of the msgs fails, the entire tx fails; abort, discard
    // uncommitted changes (the changes from the `before_tx` call earlier are
    // persisted).
    for (_idx, msg) in tx.msgs.iter().enumerate() {
        #[cfg(feature = "tracing")]
        tracing::debug!(idx = _idx, "Processing message");

        try_do! {
            process_msg(
                vm.clone(),
                Box::new(buffer.clone()),
                gas_tracker.clone(),
                block.clone(),
                tx.sender.clone(),
                msg.clone(),
            )
        }
    }

    // Call the sender account's `after_tx` method.
    //
    // If this fails, abort, discard uncommitted state changes from messages.
    // State changes from `before_tx` are always kept.
    try_do! {
        do_after_tx(
            vm,
            Box::new(buffer.clone()),
            gas_tracker.clone(),
            block,
            &tx,
            simulate,
        )
    }

    // All messages succeeded. Commit the state changes.
    buffer.write_access().commit();

    Outcome::new(gas_tracker, Ok(events))
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
        Message::Upload { code } => do_upload(&mut storage, &sender, &code),
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
        QueryRequest::Info {} => {
            let res = query_info(&storage)?;
            Ok(QueryResponse::Info(res))
        },
        QueryRequest::Balance { address, denom } => {
            let res = query_balance(vm, storage, block, gas_tracker, address, denom)?;
            Ok(QueryResponse::Balance(res))
        },
        QueryRequest::Balances {
            address,
            start_after,
            limit,
        } => {
            let res = query_balances(vm, storage, block, gas_tracker, address, start_after, limit)?;
            Ok(QueryResponse::Balances(res))
        },
        QueryRequest::Supply { denom } => {
            let res = query_supply(vm, storage, block, gas_tracker, denom)?;
            Ok(QueryResponse::Supply(res))
        },
        QueryRequest::Supplies { start_after, limit } => {
            let res = query_supplies(vm, storage, block, gas_tracker, start_after, limit)?;
            Ok(QueryResponse::Supplies(res))
        },
        QueryRequest::Code { hash } => {
            let res = query_code(&storage, hash)?;
            Ok(QueryResponse::Code(res))
        },
        QueryRequest::Codes { start_after, limit } => {
            let res = query_codes(&storage, start_after, limit)?;
            Ok(QueryResponse::Codes(res))
        },
        QueryRequest::Account { address } => {
            let res = query_account(&storage, address)?;
            Ok(QueryResponse::Account(res))
        },
        QueryRequest::Accounts { start_after, limit } => {
            let res = query_accounts(&storage, start_after, limit)?;
            Ok(QueryResponse::Accounts(res))
        },
        QueryRequest::WasmRaw { contract, key } => {
            let res = query_wasm_raw(storage, contract, key);
            Ok(QueryResponse::WasmRaw(res))
        },
        QueryRequest::WasmSmart { contract, msg } => {
            let res = query_wasm_smart(vm, storage, block, gas_tracker, contract, msg)?;
            Ok(QueryResponse::WasmSmart(res))
        },
    }
}

pub(crate) fn has_permission(permission: &Permission, owner: Option<&Addr>, sender: &Addr) -> bool {
    // The genesis sender can always store code and instantiate contracts.
    if sender == GENESIS_SENDER {
        return true;
    }

    // The owner can always do anything it wants.
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
