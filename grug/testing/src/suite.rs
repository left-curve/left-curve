use {
    grug_app::{App, AppError, AppResult, Db, Vm},
    grug_crypto::sha2_256,
    grug_db_memory::MemDb,
    grug_math::Uint128,
    grug_types::{
        Addr, Addressable, Binary, BlockInfo, BlockOutcome, Code, Coins, Config, ConfigUpdates,
        ContractInfo, Denom, Duration, GenesisState, Hash256, Json, JsonDeExt, Message, Op,
        Outcome, Query, QueryRequest, ResultExt, Signer, StdError, Tx, TxError, TxOutcome,
        TxSuccess, UnsignedTx,
    },
    grug_vm_rust::RustVm,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{collections::BTreeMap, fmt::Debug},
};

// ------------------------------- UploadOutcome -------------------------------

#[must_use = "`UploadOutcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct UploadOutcome {
    code_hash: Hash256,
    outcome: TxOutcome,
}

pub struct UploadOutcomeSuccess {
    pub code_hash: Hash256,
    pub outcome: TxSuccess,
}

impl ResultExt for UploadOutcome {
    type Error = TxError;
    type Success = UploadOutcomeSuccess;

    fn should_succeed(self) -> Self::Success {
        UploadOutcomeSuccess {
            code_hash: self.code_hash,
            outcome: self.outcome.should_succeed(),
        }
    }

    fn should_fail(self) -> Self::Error {
        self.outcome.should_fail()
    }
}

// ---------------------------- InstantiateOutcome -----------------------------

#[must_use = "`InstantiateOutcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct InstantiateOutcome {
    address: Addr,
    outcome: TxOutcome,
}

pub struct InstantiateOutcomeSuccess {
    pub address: Addr,
    pub outcome: TxSuccess,
}

impl ResultExt for InstantiateOutcome {
    type Error = TxError;
    type Success = InstantiateOutcomeSuccess;

    fn should_succeed(self) -> Self::Success {
        InstantiateOutcomeSuccess {
            address: self.address,
            outcome: self.outcome.should_succeed(),
        }
    }

    fn should_fail(self) -> Self::Error {
        self.outcome.should_fail()
    }
}

// ------------------------ UploadAndInstantiateOutcome ------------------------

#[must_use = "`UploadAndInstantiateOutcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct UploadAndInstantiateOutcome {
    code_hash: Hash256,
    address: Addr,
    outcome: TxOutcome,
}

pub struct UploadAndInstantiateOutcomeSuccess {
    pub address: Addr,
    pub code_hash: Hash256,
    pub outcome: TxSuccess,
}

impl ResultExt for UploadAndInstantiateOutcome {
    type Error = TxError;
    type Success = UploadAndInstantiateOutcomeSuccess;

    fn should_succeed(self) -> Self::Success {
        UploadAndInstantiateOutcomeSuccess {
            address: self.address,
            code_hash: self.code_hash,
            outcome: self.outcome.should_succeed(),
        }
    }

    fn should_fail(self) -> Self::Error {
        self.outcome.should_fail()
    }
}

// --------------------------------- TestSuite ---------------------------------

pub struct TestSuite<DB = MemDb, VM = RustVm>
where
    DB: Db,
    VM: Vm,
{
    pub app: App<DB, VM>,
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

impl<VM> TestSuite<MemDb, VM>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    /// Create a new test suite with `MemDb` and the given VM.
    pub fn new_with_vm(
        vm: VM,
        chain_id: String,
        block_time: Duration,
        default_gas_limit: u64,
        genesis_block: BlockInfo,
        genesis_state: GenesisState,
    ) -> Self {
        Self::new_with_db_and_vm(
            MemDb::new(),
            vm,
            chain_id,
            block_time,
            default_gas_limit,
            genesis_block,
            genesis_state,
        )
    }
}

impl<DB, VM> TestSuite<DB, VM>
where
    DB: Db,
    VM: Vm + Clone,
    AppError: From<DB::Error> + From<VM::Error>,
{
    /// Create a new test suite with the given DB and VM.
    pub fn new_with_db_and_vm(
        db: DB,
        vm: VM,
        chain_id: String,
        block_time: Duration,
        default_gas_limit: u64,
        genesis_block: BlockInfo,
        genesis_state: GenesisState,
    ) -> Self {
        // Use `u64::MAX` as query gas limit so that there's practically no limit.
        let app = App::new(db, vm, u64::MAX);

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
    pub fn check_tx(&self, tx: Tx) -> Outcome {
        self.app
            .do_check_tx(tx)
            .unwrap_or_else(|err| panic!("fatal error while checking tx: {err}"))
    }

    /// Make a new block without any transaction.
    pub fn make_empty_block(&mut self) -> BlockOutcome {
        self.make_block(vec![])
    }

    /// Make a new block with the given transactions.
    pub fn make_block(&mut self, txs: Vec<Tx>) -> BlockOutcome {
        let num_txs = txs.len();

        // Advance block height and time
        self.block.height += 1;
        self.block.timestamp = self.block.timestamp + self.block_time;

        // Call ABCI `FinalizeBlock` method
        let block_outcome = self
            .app
            .do_finalize_block(self.block, txs)
            .unwrap_or_else(|err| {
                panic!("fatal error while finalizing block: {err}");
            });

        // Sanity check: the number of tx results returned by the app should
        // equal the number of txs.
        assert_eq!(
            num_txs,
            block_outcome.tx_outcomes.len(),
            "sent {} txs but received {} tx results; something is wrong",
            num_txs,
            block_outcome.tx_outcomes.len()
        );

        // Call ABCI `Commit` method
        self.app.do_commit().unwrap_or_else(|err| {
            panic!("fatal error while committing block: {err}");
        });

        block_outcome
    }

    /// Execute a single transaction.
    pub fn send_transaction(&mut self, tx: Tx) -> TxOutcome {
        let mut block_outcome = self.make_block(vec![tx]);

        // Sanity check: we sent one transaction, so there should be exactly one
        // transaction outcome in the block outcome.
        assert_eq!(
            block_outcome.tx_outcomes.len(),
            1,
            "expecting exactly one transaction outcome, got {}; something is wrong!",
            block_outcome.tx_outcomes.len()
        );

        block_outcome.tx_outcomes.pop().unwrap()
    }

    /// Sign a transaction with the default gas limit.
    pub fn sign_transaction(&self, signer: &mut dyn Signer, msgs: Vec<Message>) -> Tx {
        self.sign_transaction_with_gas(signer, self.default_gas_limit, msgs)
    }

    /// Sign a transaction with the given gas limit.
    pub fn sign_transaction_with_gas(
        &self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        msgs: Vec<Message>,
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
        self.send_messages_with_gas(signer, gas_limit, vec![msg])
    }

    /// Execute one or more messages.
    pub fn send_messages(&mut self, signer: &mut dyn Signer, msgs: Vec<Message>) -> TxOutcome {
        self.send_messages_with_gas(signer, self.default_gas_limit, msgs)
    }

    /// Execute one or more messages under the given gas limit.
    pub fn send_messages_with_gas(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        msgs: Vec<Message>,
    ) -> TxOutcome {
        self.send_transaction(self.sign_transaction_with_gas(signer, gas_limit, msgs))
    }

    /// Update the chain's config.
    pub fn configure(
        &mut self,
        signer: &mut dyn Signer,
        updates: ConfigUpdates,
        app_updates: BTreeMap<String, Op<Json>>,
    ) -> TxOutcome {
        self.configure_with_gas(signer, self.default_gas_limit, updates, app_updates)
    }

    /// Update the chain's config under the given gas limit.
    pub fn configure_with_gas(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        updates: ConfigUpdates,
        app_updates: BTreeMap<String, Op<Json>>,
    ) -> TxOutcome {
        self.send_message_with_gas(signer, gas_limit, Message::configure(updates, app_updates))
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

        let outcome = self.send_messages_with_gas(signer, gas_limit, vec![
            Message::upload(code),
            Message::instantiate(code_hash, msg, salt, label, admin, funds).unwrap(),
        ]);

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

    pub fn query_config(&self) -> AppResult<Config> {
        self.app
            .do_query_app(Query::config(), 0, false)
            .map(|val| val.as_config())
    }

    pub fn query_app_config(&self, key: &str) -> AppResult<Json> {
        self.app
            .do_query_app(Query::app_config(key), 0, false)
            .map(|res| res.as_app_config())
    }

    pub fn query_app_configs(&self) -> AppResult<BTreeMap<String, Json>> {
        self.app
            .do_query_app(Query::app_configs(None, Some(u32::MAX)), 0, false)
            .map(|res| res.as_app_configs())
    }

    pub fn query_balance<D>(&self, account: &dyn Addressable, denom: D) -> AppResult<Uint128>
    where
        D: TryInto<Denom>,
        D::Error: Debug,
    {
        self.app
            .do_query_app(
                Query::balance(account.address(), denom.try_into().unwrap()),
                0, // zero means to use the latest height
                false,
            )
            .map(|res| res.as_balance().amount)
    }

    pub fn query_balances(&self, account: &dyn Addressable) -> AppResult<Coins> {
        self.app
            .do_query_app(
                Query::balances(account.address(), None, Some(u32::MAX)),
                0, // zero means to use the latest height
                false,
            )
            .map(|res| res.as_balances())
    }

    pub fn query_supply<D>(&self, denom: D) -> AppResult<Uint128>
    where
        D: TryInto<Denom>,
        D::Error: Debug,
    {
        self.app
            .do_query_app(Query::supply(denom.try_into().unwrap()), 0, false)
            .map(|res| res.as_supply().amount)
    }

    pub fn query_supplies(&self) -> AppResult<Coins> {
        self.app
            .do_query_app(Query::supplies(None, Some(u32::MAX)), 0, false)
            .map(|res| res.as_supplies())
    }

    pub fn query_code(&self, hash: Hash256) -> AppResult<Code> {
        self.app
            .do_query_app(Query::code(hash), 0, false)
            .map(|res| res.as_code())
    }

    pub fn query_codes(&self) -> AppResult<BTreeMap<Hash256, Code>> {
        self.app
            .do_query_app(Query::codes(None, Some(u32::MAX)), 0, false)
            .map(|res| res.as_codes())
    }

    pub fn query_contract(&self, contract: &dyn Addressable) -> AppResult<ContractInfo> {
        self.app
            .do_query_app(Query::contract(contract.address()), 0, false)
            .map(|res| res.as_contract())
    }

    pub fn query_contracts(&self) -> AppResult<BTreeMap<Addr, ContractInfo>> {
        self.app
            .do_query_app(Query::contracts(None, Some(u32::MAX)), 0, false)
            .map(|res| res.as_contracts())
    }

    pub fn query_wasm_raw<B>(&self, contract: Addr, key: B) -> AppResult<Option<Binary>>
    where
        B: Into<Binary>,
    {
        self.app
            .do_query_app(Query::wasm_raw(contract, key), 0, false)
            .map(|res| res.as_wasm_raw())
    }

    pub fn query_wasm_smart<R>(&self, contract: Addr, req: R) -> AppResult<R::Response>
    where
        R: QueryRequest,
        R::Message: Serialize,
        R::Response: DeserializeOwned + Debug,
    {
        let msg = R::Message::from(req);

        self.app
            .do_query_app(Query::wasm_smart(contract, &msg)?, 0, false)
            .map(|res| res.as_wasm_smart().deserialize_json().unwrap())
    }
}
