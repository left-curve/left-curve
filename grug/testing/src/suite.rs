use {
    anyhow::ensure,
    grug_app::{App, AppError, Db, Vm},
    grug_crypto::sha2_256,
    grug_db_memory::MemDb,
    grug_math::Uint128,
    grug_types::{
        Addr, Addressable, Binary, BlockInfo, BlockOutcome, Coins, Config, ConfigUpdates,
        ContractInfo, Denom, Duration, GenesisState, Hash256, Json, JsonDeExt, JsonSerExt, Message,
        Op, Outcome, Query, QueryRequest, ResultExt, Salt, Signer, StdError, Tx, TxOutcome,
        UnsignedTx,
    },
    grug_vm_rust::RustVm,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{collections::BTreeMap, error::Error, fmt::Debug},
};

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
    default_gas_limit: u64,
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
    ) -> anyhow::Result<Self> {
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
    ) -> anyhow::Result<Self> {
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
    ) -> anyhow::Result<Self> {
        // Use `u64::MAX` as query gas limit so that there's practically no limit.
        let app = App::new(db, vm, u64::MAX);

        app.do_init_chain(chain_id.clone(), genesis_block, genesis_state)?;

        Ok(Self {
            app,
            chain_id,
            block: genesis_block,
            block_time,
            default_gas_limit,
        })
    }

    /// Simulate the gas cost and event outputs of an unsigned transaction.
    pub fn simulate_tx(&self, unsigned_tx: UnsignedTx) -> anyhow::Result<TxOutcome> {
        Ok(self.app.do_simulate(unsigned_tx, 0, false)?)
    }

    /// Perform ABCI `CheckTx` call of a transaction.
    pub fn check_tx(&self, tx: Tx) -> anyhow::Result<Outcome> {
        Ok(self.app.do_check_tx(tx)?)
    }

    /// Make a new block without any transaction.
    pub fn make_empty_block(&mut self) -> anyhow::Result<BlockOutcome> {
        self.make_block(vec![])
    }

    /// Make a new block with the given transactions.
    pub fn make_block(&mut self, txs: Vec<Tx>) -> anyhow::Result<BlockOutcome> {
        let num_txs = txs.len();

        // Advance block height and time
        self.block.height += 1;
        self.block.timestamp = self.block.timestamp + self.block_time;

        // Call ABCI `FinalizeBlock` method
        let block_outcome = self.app.do_finalize_block(self.block, txs)?;

        // Sanity check: the number of tx results returned by the app should
        // equal the number of txs.
        ensure!(
            num_txs == block_outcome.tx_outcomes.len(),
            "sent {} txs but received {} tx results; something is wrong",
            num_txs,
            block_outcome.tx_outcomes.len()
        );

        // Call ABCI `Commit` method
        self.app.do_commit()?;

        Ok(block_outcome)
    }

    /// Execute a single transaction.
    pub fn send_transaction(&mut self, tx: Tx) -> anyhow::Result<TxOutcome> {
        let mut block_outcome = self.make_block(vec![tx])?;

        // Sanity check: we sent one transaction, so there should be exactly one
        // transaction outcome in the block outcome.
        ensure!(
            block_outcome.tx_outcomes.len() == 1,
            "expecting exactly one transaction outcome, got {}; something is wrong!",
            block_outcome.tx_outcomes.len()
        );

        Ok(block_outcome.tx_outcomes.pop().unwrap())
    }

    /// Execute a single message.
    pub fn send_message(
        &mut self,
        signer: &mut dyn Signer,
        msg: Message,
    ) -> anyhow::Result<TxOutcome> {
        self.send_message_with_gas(signer, self.default_gas_limit, msg)
    }

    /// Execute a single message under the given gas limit.
    pub fn send_message_with_gas(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        msg: Message,
    ) -> anyhow::Result<TxOutcome> {
        self.send_messages_with_gas(signer, gas_limit, vec![msg])
    }

    /// Execute one or more messages.
    pub fn send_messages(
        &mut self,
        signer: &mut dyn Signer,
        msgs: Vec<Message>,
    ) -> anyhow::Result<TxOutcome> {
        self.send_messages_with_gas(signer, self.default_gas_limit, msgs)
    }

    /// Execute one or more messages under the given gas limit.
    pub fn send_messages_with_gas(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        msgs: Vec<Message>,
    ) -> anyhow::Result<TxOutcome> {
        ensure!(!msgs.is_empty(), "please send more than zero messages");

        // Compose and sign a single message
        let tx = signer.sign_transaction(msgs, &self.chain_id, gas_limit)?;

        self.send_transaction(tx)
    }

    /// Update the chain's config.
    pub fn configure(
        &mut self,
        signer: &mut dyn Signer,
        updates: ConfigUpdates,
        app_updates: BTreeMap<String, Op<Json>>,
    ) -> anyhow::Result<()> {
        self.configure_with_gas(signer, self.default_gas_limit, updates, app_updates)
    }

    /// Update the chain's config under the given gas limit.
    pub fn configure_with_gas(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        updates: ConfigUpdates,
        app_updates: BTreeMap<String, Op<Json>>,
    ) -> anyhow::Result<()> {
        self.send_message_with_gas(signer, gas_limit, Message::configure(updates, app_updates))?
            .result
            .should_succeed();

        Ok(())
    }

    /// Make a transfer of tokens.
    pub fn transfer<C>(&mut self, signer: &mut dyn Signer, to: Addr, coins: C) -> anyhow::Result<()>
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
    ) -> anyhow::Result<()>
    where
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.send_message_with_gas(signer, gas_limit, Message::transfer(to, coins)?)?
            .result
            .should_succeed();

        Ok(())
    }

    /// Upload a code. Return the code's hash.
    pub fn upload<B>(&mut self, signer: &mut dyn Signer, code: B) -> anyhow::Result<Hash256>
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
    ) -> anyhow::Result<Hash256>
    where
        B: Into<Binary>,
    {
        let code = code.into();
        let code_hash = Hash256::from_array(sha2_256(&code));

        self.send_message_with_gas(signer, gas_limit, Message::upload(code))?
            .result
            .should_succeed();

        Ok(code_hash)
    }

    /// Instantiate a contract. Return the contract's address.
    pub fn instantiate<M, S, C>(
        &mut self,
        signer: &mut dyn Signer,
        code_hash: Hash256,
        salt: S,
        msg: &M,
        funds: C,
    ) -> anyhow::Result<Addr>
    where
        M: Serialize,
        S: Into<Salt>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.instantiate_with_gas(signer, self.default_gas_limit, code_hash, salt, msg, funds)
    }

    /// Instantiate a contract under the given gas limit. Return the contract's
    /// address.
    pub fn instantiate_with_gas<M, S, C>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        code_hash: Hash256,
        salt: S,
        msg: &M,
        funds: C,
    ) -> anyhow::Result<Addr>
    where
        M: Serialize,
        S: Into<Salt>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let salt = salt.into();
        let address = Addr::compute(signer.address(), code_hash, &salt);

        self.send_message_with_gas(
            signer,
            gas_limit,
            Message::instantiate(code_hash, msg, salt, funds, None)?,
        )?
        .result
        .should_succeed();

        Ok(address)
    }

    /// Upload a code and instantiate a contract with it in one go. Return the
    /// code hash as well as the contract's address.
    pub fn upload_and_instantiate<M, B, S, C>(
        &mut self,
        signer: &mut dyn Signer,
        code: B,
        salt: S,
        msg: &M,
        funds: C,
    ) -> anyhow::Result<(Hash256, Addr)>
    where
        M: Serialize,
        B: Into<Binary>,
        S: Into<Salt>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.upload_and_instantiate_with_gas(signer, self.default_gas_limit, code, salt, msg, funds)
    }

    /// Upload a code and instantiate a contract with it in one go under the
    /// given gas limit. Return the code hash as well as the contract's address.
    pub fn upload_and_instantiate_with_gas<M, B, S, C>(
        &mut self,
        signer: &mut dyn Signer,
        gas_limit: u64,
        code: B,
        salt: S,
        msg: &M,
        funds: C,
    ) -> anyhow::Result<(Hash256, Addr)>
    where
        M: Serialize,
        B: Into<Binary>,
        S: Into<Salt>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let code = code.into();
        let code_hash = Hash256::from_array(sha2_256(&code));
        let salt = salt.into();
        let address = Addr::compute(signer.address(), code_hash, &salt);

        self.send_messages_with_gas(signer, gas_limit, vec![
            Message::upload(code),
            Message::instantiate(code_hash, msg, salt, funds, None)?,
        ])?
        .result
        .should_succeed();

        Ok((code_hash, address))
    }

    /// Execute a contrat.
    pub fn execute<M, C>(
        &mut self,
        signer: &mut dyn Signer,
        contract: Addr,
        msg: &M,
        funds: C,
    ) -> anyhow::Result<()>
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
    ) -> anyhow::Result<()>
    where
        M: Serialize,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        self.send_message_with_gas(signer, gas_limit, Message::execute(contract, msg, funds)?)?
            .result
            .should_succeed();

        Ok(())
    }

    /// Migrate a contract to a new code hash.
    pub fn migrate<M>(
        &mut self,
        signer: &mut dyn Signer,
        contract: Addr,
        new_code_hash: Hash256,
        msg: &M,
    ) -> anyhow::Result<()>
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
    ) -> anyhow::Result<()>
    where
        M: Serialize,
    {
        self.send_message_with_gas(
            signer,
            gas_limit,
            Message::migrate(contract, new_code_hash, msg)?,
        )?
        .result
        .should_succeed();

        Ok(())
    }

    pub fn query_config(&self) -> anyhow::Result<Config> {
        self.app
            .do_query_app(Query::Config {}, 0, false)
            .map(|val| val.as_config())
            .map_err(Into::into)
    }

    pub fn query_app_config(&self, key: &str) -> anyhow::Result<Json> {
        self.app
            .do_query_app(
                Query::AppConfig {
                    key: key.to_string(),
                },
                0,
                false,
            )
            .map(|res| res.as_app_config())
            .map_err(Into::into)
    }

    pub fn query_app_configs(&self) -> anyhow::Result<BTreeMap<String, Json>> {
        self.app
            .do_query_app(
                Query::AppConfigs {
                    start_after: None,
                    limit: Some(u32::MAX),
                },
                0,
                false,
            )
            .map(|res| res.as_app_configs())
            .map_err(Into::into)
    }

    pub fn query_balance<D>(&self, account: &dyn Addressable, denom: D) -> anyhow::Result<Uint128>
    where
        D: TryInto<Denom>,
        D::Error: Error + Send + Sync + 'static,
    {
        let denom = denom.try_into()?;

        self.app
            .do_query_app(
                Query::Balance {
                    address: account.address(),
                    denom,
                },
                0, // zero means to use the latest height
                false,
            )
            .map(|res| res.as_balance().amount)
            .map_err(Into::into)
    }

    pub fn query_balances(&self, account: &dyn Addressable) -> anyhow::Result<Coins> {
        self.app
            .do_query_app(
                Query::Balances {
                    address: account.address(),
                    start_after: None,
                    limit: Some(u32::MAX),
                },
                0, // zero means to use the latest height
                false,
            )
            .map(|res| res.as_balances())
            .map_err(Into::into)
    }

    pub fn query_supply<D>(&self, denom: D) -> anyhow::Result<Uint128>
    where
        D: TryInto<Denom>,
        D::Error: Error + Send + Sync + 'static,
    {
        let denom = denom.try_into()?;

        self.app
            .do_query_app(Query::Supply { denom }, 0, false)
            .map(|res| res.as_supply().amount)
            .map_err(Into::into)
    }

    pub fn query_supplies(&self) -> anyhow::Result<Coins> {
        self.app
            .do_query_app(
                Query::Supplies {
                    start_after: None,
                    limit: Some(u32::MAX),
                },
                0,
                false,
            )
            .map(|res| res.as_supplies())
            .map_err(Into::into)
    }

    pub fn query_code(&self, hash: Hash256) -> anyhow::Result<Binary> {
        self.app
            .do_query_app(Query::Code { hash }, 0, false)
            .map(|res| res.as_code())
            .map_err(Into::into)
    }

    pub fn query_codes(&self) -> anyhow::Result<BTreeMap<Hash256, Binary>> {
        self.app
            .do_query_app(
                Query::Codes {
                    start_after: None,
                    limit: Some(u32::MAX),
                },
                0,
                false,
            )
            .map(|res| res.as_codes())
            .map_err(Into::into)
    }

    pub fn query_contract(&self, contract: &dyn Addressable) -> anyhow::Result<ContractInfo> {
        self.app
            .do_query_app(
                Query::Contract {
                    address: contract.address(),
                },
                0,
                false,
            )
            .map(|res| res.as_contract())
            .map_err(Into::into)
    }

    pub fn query_contracts(&self) -> anyhow::Result<BTreeMap<Addr, ContractInfo>> {
        self.app
            .do_query_app(
                Query::Contracts {
                    start_after: None,
                    limit: Some(u32::MAX),
                },
                0,
                false,
            )
            .map(|res| res.as_contracts())
            .map_err(Into::into)
    }

    pub fn query_wasm_raw<B>(&self, contract: Addr, key: B) -> anyhow::Result<Option<Binary>>
    where
        B: Into<Binary>,
    {
        self.app
            .do_query_app(
                Query::WasmRaw {
                    contract,
                    key: key.into(),
                },
                0,
                false,
            )
            .map(|res| res.as_wasm_raw())
            .map_err(Into::into)
    }

    pub fn query_wasm_smart<R>(&self, contract: Addr, req: R) -> anyhow::Result<R::Response>
    where
        R: QueryRequest,
        R::Message: Serialize,
        R::Response: DeserializeOwned + Debug,
    {
        let msg = R::Message::from(req);
        let msg_raw = msg.to_json_value()?;

        self.app
            .do_query_app(
                Query::WasmSmart {
                    contract,
                    msg: msg_raw,
                },
                0, // zero means to use the latest height
                false,
            )?
            .as_wasm_smart()
            .deserialize_json()
            .map_err(Into::into)
    }
}
