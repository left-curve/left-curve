use {
    crate::{TestAccount, TestResult},
    anyhow::ensure,
    grug_app::{App, AppError, AppResult, Vm},
    grug_crypto::sha2_256,
    grug_db_memory::MemDb,
    grug_types::{
        from_json_value, to_json_value, Addr, Binary, BlockInfo, Coins, Event, GenesisState, Hash,
        InfoResponse, Message, NumberConst, QueryRequest, Uint128, Uint64,
    },
    grug_vm_rust::RustVm,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{collections::HashMap, time::Duration},
};

pub struct TestSuite<VM: Vm = RustVm> {
    app: App<MemDb, VM>,
    /// The chain ID can be queries from the `app`, but we internally track it in
    /// the test suite, so we don't need to query it every time we need it.
    chain_id: String,
    /// Interally track the last finalized block.
    block: BlockInfo,
    /// Each time we make a new block, we set the new block's time as the
    /// previous block's time plus this value.
    block_time: Duration,
    /// Internally track each account's sequence number.
    sequences: HashMap<Addr, u32>,
}

impl<VM> TestSuite<VM>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    /// Only exposed to the crate. Use `TestBuilder` instead.
    pub(crate) fn create(
        vm: VM,
        chain_id: String,
        block_time: Duration,
        genesis_block: BlockInfo,
        genesis_state: GenesisState,
    ) -> anyhow::Result<Self> {
        let app = App::new(MemDb::new(), vm, None);

        app.do_init_chain(chain_id.clone(), genesis_block.clone(), genesis_state)?;

        Ok(Self {
            app,
            chain_id,
            block: genesis_block,
            block_time,
            sequences: HashMap::new(),
        })
    }

    /// Execute a single message under the given gas limit.
    pub fn execute_message_with_gas(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        msg: Message,
    ) -> anyhow::Result<TestResult<Vec<Event>>> {
        self.execute_messages_with_gas(signer, gas_limit, vec![msg])
    }

    /// Execute one or more messages under the given gas limit.
    pub fn execute_messages_with_gas(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        msgs: Vec<Message>,
    ) -> anyhow::Result<TestResult<Vec<Event>>> {
        // Get the account's sequence
        let sequence = self.sequences.entry(signer.address.clone()).or_insert(0);
        // Sign the transaction
        let tx = signer.sign_transaction(msgs.clone(), gas_limit, &self.chain_id, *sequence)?;
        // Increment the sequence
        *sequence += 1;

        // Make a new block
        self.block.height += Uint64::ONE;
        self.block.timestamp = self.block.timestamp.plus_nanos(self.block_time.as_nanos());

        // Finalize the block
        let (_, _, mut results) = self
            .app
            .do_finalize_block(self.block.clone(), vec![(Hash::ZERO, tx)])?;

        // We only sent 1 transaction, so there should be exactly one tx result
        ensure!(
            results.len() == 1,
            "received {} tx results; something is wrong",
            results.len()
        );

        // Commit state changes
        self.app.do_commit()?;

        Ok(results.pop().unwrap().into())
    }

    /// Upload a code under the given gas limit. Return the code's hash.
    pub fn upload_with_gas(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        code: Binary,
    ) -> anyhow::Result<Hash> {
        let code_hash = Hash::from_slice(sha2_256(&code));

        self.execute_message_with_gas(signer, gas_limit, Message::upload(code))?
            .should_succeed()?;

        Ok(code_hash)
    }

    /// Instantiate a contract under the given gas limit. Return the contract's
    /// address.
    pub fn instantiate_with_gas<M, S>(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        code_hash: Hash,
        salt: S,
        msg: &M,
    ) -> anyhow::Result<Addr>
    where
        M: Serialize,
        S: Into<Binary>,
    {
        let salt = salt.into();
        let address = Addr::compute(&signer.address, &code_hash, &salt);

        self.execute_message_with_gas(
            signer,
            gas_limit,
            Message::instantiate(code_hash, msg, salt, Coins::new_empty(), None)?,
        )?
        .should_succeed()?;

        Ok(address)
    }

    /// Upload a code and instantiate a contract with it in one go under the
    /// given gas limit. Return the code hash as well as the contract's address.
    pub fn upload_and_instantiate_with_gas<M, S>(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        code: Binary,
        salt: S,
        msg: &M,
    ) -> anyhow::Result<(Hash, Addr)>
    where
        M: Serialize,
        S: Into<Binary>,
    {
        let salt = salt.into();
        let code_hash = Hash::from_slice(sha2_256(&code));
        let address = Addr::compute(&signer.address, &code_hash, &salt);

        self.execute_messages_with_gas(signer, gas_limit, vec![
            Message::upload(code),
            Message::instantiate(code_hash.clone(), msg, salt, Coins::new_empty(), None)?,
        ])?
        .should_succeed()?;

        Ok((code_hash, address))
    }

    pub fn query_wasm_smart<M, R>(&self, contract: Addr, msg: &M) -> TestResult<R>
    where
        M: Serialize,
        R: DeserializeOwned,
    {
        (|| -> AppResult<_> {
            let msg_raw = to_json_value(msg)?;
            let res_raw = self
                .app
                .do_query_app(
                    QueryRequest::WasmSmart {
                        contract,
                        msg: msg_raw,
                    },
                    0, // zero means to use the latest height
                    false,
                )?
                .as_wasm_smart()
                .data;
            Ok(from_json_value(res_raw)?)
        })()
        .into()
    }

    pub fn query_balance(&self, account: &TestAccount, denom: &str) -> TestResult<Uint128> {
        self.app
            .do_query_app(
                QueryRequest::Balance {
                    address: account.address.clone(),
                    denom: denom.to_string(),
                },
                0, // zero means to use the latest height
                false,
            )
            .map(|res| res.as_balance().amount)
            .into()
    }

    pub fn query_info(&self) -> TestResult<InfoResponse> {
        self.app
            .do_query_app(QueryRequest::Info {}, 0, false)
            .map(|val| val.as_info())
            .into()
    }
}

// Rust VM doesn't support gas, so we introduce these convenience methods that
// don't take a `gas_limit` parameter.
impl TestSuite<RustVm> {
    /// Execute a single message.
    pub fn execute_message(
        &mut self,
        signer: &TestAccount,
        msg: Message,
    ) -> anyhow::Result<TestResult<Vec<Event>>> {
        self.execute_message_with_gas(signer, 0, msg)
    }

    /// Execute one or more messages.
    pub fn execute_messages(
        &mut self,
        signer: &TestAccount,
        msgs: Vec<Message>,
    ) -> anyhow::Result<TestResult<Vec<Event>>> {
        self.execute_messages_with_gas(signer, 0, msgs)
    }

    /// Upload a code. Return the code's hash.
    pub fn upload(&mut self, signer: &TestAccount, code: Binary) -> anyhow::Result<Hash> {
        self.upload_with_gas(signer, 0, code)
    }

    /// Instantiate a contract. Return the contract's address.
    pub fn instantiate<M, S>(
        &mut self,
        signer: &TestAccount,
        code_hash: Hash,
        salt: S,
        msg: &M,
    ) -> anyhow::Result<Addr>
    where
        M: Serialize,
        S: Into<Binary>,
    {
        self.instantiate_with_gas(signer, 0, code_hash, salt, msg)
    }

    /// Upload a code and instantiate a contract with it in one go. Return the
    /// code hash as well as the contract's address.
    pub fn upload_and_instantiate<M, S>(
        &mut self,
        signer: &TestAccount,
        code: Binary,
        salt: S,
        msg: &M,
    ) -> anyhow::Result<(Hash, Addr)>
    where
        M: Serialize,
        S: Into<Binary>,
    {
        self.upload_and_instantiate_with_gas(signer, 0, code, salt, msg)
    }
}
