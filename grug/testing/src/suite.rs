use {
    crate::{
        BalanceTracker, InstantiateOutcome, MakeBlockOutcome, UploadAndInstantiateOutcome,
        UploadOutcome,
    },
    grug_app::{
        App, AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, ProposalPreparer, Vm,
    },
    grug_crypto::sha2_256,
    grug_db_memory::MemDb,
    grug_math::Uint128,
    grug_types::{
        Addr, Addressable, Binary, Block, BlockInfo, CheckTxOutcome, Coins, Config, Denom,
        Duration, GenesisState, Hash256, HashExt, JsonDeExt, JsonSerExt, Message, NonEmpty,
        Querier, QuerierExt, QuerierWrapper, Query, QueryResponse, Signer, StdError, StdResult, Tx,
        TxOutcome, UnsignedTx,
    },
    grug_vm_rust::RustVm,
    serde::ser::Serialize,
    std::collections::BTreeMap,
};

pub struct TestSuite<DB = MemDb, VM = RustVm, PP = NaiveProposalPreparer, ID = NullIndexer>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
{
    pub app: App<DB, VM, PP, ID>,
    /// The chain ID can be queries from the `app`, but we internally track it in
    /// the test suite, so we don't need to query it every time we need it.
    pub chain_id: String,
    /// Interally track the last finalized block.
    pub block: BlockInfo,
    /// Each time we make a new block, we set the new block's time as the
    /// previous block's time plus this value.
    pub block_time: Duration,
    /// Transaction gas limit to use if user doesn't specify one.
    pub default_gas_limit: u64,
    pub(crate) balances: BTreeMap<Addr, Coins>,
}

impl TestSuite {
    /// Create a new test suite.
    ///
    /// It's not recommended to call this directly. Use [`TestBuilder`](crate::TestBuilder)
    /// instead.
    pub fn new(
        chain_id: String,
        block_time: Duration,
        default_gas_limit: u64,
        genesis_block: BlockInfo,
        genesis_state: GenesisState,
    ) -> Self {
        Self::new_with_vm(
            RustVm::new(),
            chain_id,
            block_time,
            default_gas_limit,
            genesis_block,
            genesis_state,
        )
    }
}

impl<VM> TestSuite<MemDb, VM, NaiveProposalPreparer, NullIndexer>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    /// Create a new test suite with `MemDb`, `NaiveProposalPreparer`, and the
    /// given VM.
    pub fn new_with_vm(
        vm: VM,
        chain_id: String,
        block_time: Duration,
        default_gas_limit: u64,
        genesis_block: BlockInfo,
        genesis_state: GenesisState,
    ) -> Self {
        Self::new_with_db_vm_indexer_and_pp(
            MemDb::new(),
            vm,
            NaiveProposalPreparer,
            NullIndexer,
            chain_id,
            block_time,
            default_gas_limit,
            genesis_block,
            genesis_state,
        )
    }
}

impl<PP> TestSuite<MemDb, RustVm, PP, NullIndexer>
where
    PP: ProposalPreparer,
    AppError: From<PP::Error>,
{
    /// Create a new test suite with `MemDb`, `RustVm`, and the given proposal
    /// preparer.
    pub fn new_with_pp(
        pp: PP,
        chain_id: String,
        block_time: Duration,
        default_gas_limit: u64,
        genesis_block: BlockInfo,
        genesis_state: GenesisState,
    ) -> Self {
        Self::new_with_db_vm_indexer_and_pp(
            MemDb::new(),
            RustVm::new(),
            pp,
            NullIndexer,
            chain_id,
            block_time,
            default_gas_limit,
            genesis_block,
            genesis_state,
        )
    }
}

impl<DB, VM, PP, ID> TestSuite<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
{
    /// Create a new test suite with the given DB and VM.
    pub fn new_with_db_vm_indexer_and_pp(
        db: DB,
        vm: VM,
        pp: PP,
        mut id: ID,
        chain_id: String,
        block_time: Duration,
        default_gas_limit: u64,
        genesis_block: BlockInfo,
        genesis_state: GenesisState,
    ) -> Self {
        // This is doing the same order as in Dango.
        // 1. Calling `start` on the indexer

        let previous_block_height = if let 0 | 1 = genesis_block.height {
            None
        } else {
            Some(genesis_block.height - 1)
        };

        let state_storage = db
            .state_storage(previous_block_height)
            .unwrap_or_else(|err| {
                panic!(
                    "Fatal error while getting the state storage: {}",
                    err.to_string()
                );
            });

        id.start(&state_storage).unwrap_or_else(|err| {
            panic!("fatal error while running indexer start: {err}");
        });

        // 2. Creating the app instance
        // Use `u64::MAX` as query gas limit so that there's practically no limit.
        let app = App::new(db, vm, pp, id, u64::MAX);

        app.do_init_chain(chain_id.clone(), genesis_block, genesis_state)
            .unwrap_or_else(|err| {
                panic!("fatal error while initializing chain: {err}");
            });

        Self {
            app,
            chain_id,
            block: genesis_block,
            block_time,
            default_gas_limit,
            balances: Default::default(),
        }
    }

    /// Simulate the gas cost and event outputs of an unsigned transaction.
    pub fn simulate_tx(&self, unsigned_tx: UnsignedTx) -> TxOutcome {
        self.app
            .do_simulate(unsigned_tx, 0, false)
            .unwrap_or_else(|err| {
                panic!("fatal error while simulating tx: {err}");
            })
    }

    /// Perform ABCI `CheckTx` call of a transaction.
    pub fn check_tx(&self, tx: Tx) -> CheckTxOutcome {
        self.app
            .do_check_tx(tx)
            .unwrap_or_else(|err| panic!("fatal error while checking tx: {err}"))
    }

    /// Increase the chain's time by the given duration.
    pub fn increase_time(&mut self, duration: Duration) {
        let old_block_time = self.block_time;
        self.block_time = duration;
        self.make_empty_block();
        self.block_time = old_block_time;
    }

    /// Make a new block without any transaction.
    pub fn make_empty_block(&mut self) -> MakeBlockOutcome {
        self.make_block(vec![])
    }

    /// Make a new block with the given transactions.
    pub fn make_block(&mut self, txs: Vec<Tx>) -> MakeBlockOutcome {
        // Advance block height and time
        self.block.height += 1;
        self.block.timestamp = self.block.timestamp + self.block_time;

        // Prepare proposal
        let raw_txs = txs
            .into_iter()
            .map(|tx| tx.to_json_vec().unwrap().into())
            .collect();
        let txs = self
            .app
            .do_prepare_proposal(raw_txs, usize::MAX)
            .into_iter()
            .map(|raw_tx| (raw_tx.deserialize_json().unwrap(), raw_tx.hash256()))
            .collect::<Vec<_>>();

        let block = Block {
            info: self.block,
            txs: txs.clone(),
        };

        // Call ABCI `FinalizeBlock` method
        let block_outcome = self.app.do_finalize_block(block).unwrap_or_else(|err| {
            panic!("fatal error while finalizing block: {err}");
        });

        // Call ABCI `Commit` method
        self.app.do_commit().unwrap_or_else(|err| {
            panic!("fatal error while committing block: {err}");
        });

        MakeBlockOutcome { txs, block_outcome }
    }

    /// Execute a single transaction.
    pub fn send_transaction(&mut self, tx: Tx) -> TxOutcome {
        let mut block_outcome = self.make_block(vec![tx]);

        block_outcome.block_outcome.tx_outcomes.pop().unwrap()
    }

    /// Sign a transaction with the default gas limit.
    pub fn sign_transaction(&self, signer: &mut dyn Signer, msgs: NonEmpty<Vec<Message>>) -> Tx {
        self.sign_transaction_with_gas(signer, self.default_gas_limit, msgs)
    }

    /// Sign a transaction with the given gas limit.
    pub fn sign_transaction_with_gas(
        &self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        msgs: NonEmpty<Vec<Message>>,
    ) -> Tx {
        signer
            .sign_transaction(msgs, &self.chain_id, gas_limit)
            .unwrap_or_else(|err| {
                panic!("fatal error while signing tx: {err}");
            })
    }

    /// Execute a single message.
    pub fn send_message(&mut self, signer: &mut dyn Signer, msg: Message) -> TxOutcome {
        self.send_message_with_gas(signer, self.default_gas_limit, msg)
    }

    /// Execute a single message under the given gas limit.
    pub fn send_message_with_gas(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        msg: Message,
    ) -> TxOutcome {
        self.send_messages_with_gas(signer, gas_limit, NonEmpty::new_unchecked(vec![msg]))
    }

    /// Execute one or more messages.
    pub fn send_messages(
        &mut self,
        signer: &mut dyn Signer,
        msgs: NonEmpty<Vec<Message>>,
    ) -> TxOutcome {
        self.send_messages_with_gas(signer, self.default_gas_limit, msgs)
    }

    /// Execute one or more messages under the given gas limit.
    pub fn send_messages_with_gas(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        msgs: NonEmpty<Vec<Message>>,
    ) -> TxOutcome {
        self.send_transaction(self.sign_transaction_with_gas(signer, gas_limit, msgs))
    }

    /// Update the chain's config.
    pub fn configure<T>(
        &mut self,
        signer: &mut dyn Signer,
        new_cfg: Option<Config>,
        new_app_cfg: Option<T>,
    ) -> TxOutcome
    where
        T: Serialize,
    {
        self.configure_with_gas(signer, self.default_gas_limit, new_cfg, new_app_cfg)
    }

    /// Update the chain's config under the given gas limit.
    pub fn configure_with_gas<T>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        new_cfg: Option<Config>,
        new_app_cfg: Option<T>,
    ) -> TxOutcome
    where
        T: Serialize,
    {
        self.send_message_with_gas(
            signer,
            gas_limit,
            Message::configure(new_cfg, new_app_cfg).unwrap(),
        )
    }

    /// Make a transfer of tokens.
    pub fn transfer<C>(&mut self, signer: &mut dyn Signer, to: Addr, coins: C) -> TxOutcome
    where
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.transfer_with_gas(signer, self.default_gas_limit, to, coins)
    }

    /// Make a transfer of tokens under the given gas limit.
    pub fn transfer_with_gas<C>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        to: Addr,
        coins: C,
    ) -> TxOutcome
    where
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.send_message_with_gas(signer, gas_limit, Message::transfer(to, coins).unwrap())
    }

    /// Make a batched transfer of tokens to multiple recipients.
    pub fn batch_transfer(
        &mut self,
        signer: &mut dyn Signer,
        transfers: BTreeMap<Addr, Coins>,
    ) -> TxOutcome {
        self.batch_transfer_with_gas(signer, self.default_gas_limit, transfers)
    }

    /// Make a batched transfer of tokens to multiple recipients, under the
    /// given gas limit.
    pub fn batch_transfer_with_gas(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        transfers: BTreeMap<Addr, Coins>,
    ) -> TxOutcome {
        self.send_message_with_gas(
            signer,
            gas_limit,
            Message::batch_transfer(transfers).unwrap(),
        )
    }

    /// Upload a code. Return the code's hash.
    pub fn upload<B>(&mut self, signer: &mut dyn Signer, code: B) -> UploadOutcome
    where
        B: Into<Binary>,
    {
        self.upload_with_gas(signer, self.default_gas_limit, code)
    }

    /// Upload a code under the given gas limit. Return the code's hash.
    pub fn upload_with_gas<B>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        code: B,
    ) -> UploadOutcome
    where
        B: Into<Binary>,
    {
        let code = code.into();
        let code_hash = Hash256::from_inner(sha2_256(&code));

        let outcome = self.send_message_with_gas(signer, gas_limit, Message::upload(code));

        UploadOutcome { code_hash, outcome }
    }

    /// Instantiate a contract. Return the contract's address.
    pub fn instantiate<M, S, C>(
        &mut self,
        signer: &mut dyn Signer,
        code_hash: Hash256,
        msg: &M,
        salt: S,
        label: Option<&str>,
        admin: Option<Addr>,
        funds: C,
    ) -> InstantiateOutcome
    where
        M: Serialize,
        S: Into<Binary>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.instantiate_with_gas(
            signer,
            self.default_gas_limit,
            code_hash,
            msg,
            salt,
            label,
            admin,
            funds,
        )
    }

    /// Instantiate a contract under the given gas limit. Return the contract's
    /// address.
    pub fn instantiate_with_gas<M, S, C>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        code_hash: Hash256,
        msg: &M,
        salt: S,
        label: Option<&str>,
        admin: Option<Addr>,
        funds: C,
    ) -> InstantiateOutcome
    where
        M: Serialize,
        S: Into<Binary>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let salt = salt.into();
        let address = Addr::derive(signer.address(), code_hash, &salt);

        let outcome = self.send_message_with_gas(
            signer,
            gas_limit,
            Message::instantiate(code_hash, msg, salt, label, admin, funds).unwrap(),
        );

        InstantiateOutcome { address, outcome }
    }

    /// Upload a code and instantiate a contract with it in one go. Return the
    /// code hash as well as the contract's address.
    pub fn upload_and_instantiate<M, B, S, L, C>(
        &mut self,
        signer: &mut dyn Signer,
        code: B,
        msg: &M,
        salt: S,
        label: Option<L>,
        admin: Option<Addr>,
        funds: C,
    ) -> UploadAndInstantiateOutcome
    where
        M: Serialize,
        B: Into<Binary>,
        S: Into<Binary>,
        L: Into<String>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.upload_and_instantiate_with_gas(
            signer,
            self.default_gas_limit,
            code,
            msg,
            salt,
            label,
            admin,
            funds,
        )
    }

    /// Upload a code and instantiate a contract with it in one go under the
    /// given gas limit. Return the code hash as well as the contract's address.
    pub fn upload_and_instantiate_with_gas<M, B, S, L, C>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        code: B,
        msg: &M,
        salt: S,
        label: Option<L>,
        admin: Option<Addr>,
        funds: C,
    ) -> UploadAndInstantiateOutcome
    where
        M: Serialize,
        B: Into<Binary>,
        S: Into<Binary>,
        L: Into<String>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let code = code.into();
        let code_hash = Hash256::from_inner(sha2_256(&code));
        let salt = salt.into();
        let address = Addr::derive(signer.address(), code_hash, &salt);

        let outcome = self.send_messages_with_gas(
            signer,
            gas_limit,
            NonEmpty::new_unchecked(vec![
                Message::upload(code),
                Message::instantiate(code_hash, msg, salt, label, admin, funds).unwrap(),
            ]),
        );

        UploadAndInstantiateOutcome {
            address,
            code_hash,
            outcome,
        }
    }

    /// Execute a contrat.
    pub fn execute<M, C>(
        &mut self,
        signer: &mut dyn Signer,
        contract: Addr,
        msg: &M,
        funds: C,
    ) -> TxOutcome
    where
        M: Serialize,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.execute_with_gas(signer, self.default_gas_limit, contract, msg, funds)
    }

    /// Execute a contrat under the given gas limit.
    pub fn execute_with_gas<M, C>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        contract: Addr,
        msg: &M,
        funds: C,
    ) -> TxOutcome
    where
        M: Serialize,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.send_message_with_gas(
            signer,
            gas_limit,
            Message::execute(contract, msg, funds).unwrap(),
        )
    }

    /// Migrate a contract to a new code hash.
    pub fn migrate<M>(
        &mut self,
        signer: &mut dyn Signer,
        contract: Addr,
        new_code_hash: Hash256,
        msg: &M,
    ) -> TxOutcome
    where
        M: Serialize,
    {
        self.migrate_with_gas(signer, self.default_gas_limit, contract, new_code_hash, msg)
    }

    /// Migrate a contract to a new code hash, under the given gas limit.
    pub fn migrate_with_gas<M>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        contract: Addr,
        new_code_hash: Hash256,
        msg: &M,
    ) -> TxOutcome
    where
        M: Serialize,
    {
        self.send_message_with_gas(
            signer,
            gas_limit,
            Message::migrate(contract, new_code_hash, msg).unwrap(),
        )
    }

    /// Return a `QuerierWrapper` object.
    pub fn querier(&self) -> QuerierWrapper {
        QuerierWrapper::new(self)
    }
}

impl<DB, VM, PP, ID> Querier for TestSuite<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
{
    fn query_chain(&self, req: Query) -> StdResult<QueryResponse> {
        self.app
            .do_query_app(req, 0, false)
            .map_err(|err| StdError::host(err.to_string()))
    }
}

impl<DB, VM, PP, ID> TestSuite<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
    Self: Querier,
{
    pub fn query_balance<D>(&self, address: &dyn Addressable, denom: D) -> StdResult<Uint128>
    where
        D: TryInto<Denom>,
        StdError: From<D::Error>,
    {
        let address = address.address();
        let denom = denom.try_into()?;
        <Self as QuerierExt>::query_balance(self, address, denom)
    }

    pub fn query_balances(&self, account: &dyn Addressable) -> StdResult<Coins> {
        let address = account.address();
        <Self as QuerierExt>::query_balances(self, address, None, Some(u32::MAX))
    }

    pub fn balances(&mut self) -> BalanceTracker<DB, VM, PP, ID> {
        BalanceTracker { suite: self }
    }
}
