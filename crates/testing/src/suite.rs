use {
    crate::{TestAccount, TestResult},
    anyhow::ensure,
    grug_app::{App, AppError, AppResult, Vm},
    grug_crypto::sha2_256,
    grug_db_memory::MemDb,
    grug_types::{
        from_json_value, to_json_value, Addr, Binary, BlockInfo, Coins, Event, GenesisState, Hash,
        Message, NumberConst, QueryRequest, Uint128, Uint64,
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

    pub fn execute_message(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        msg: Message,
    ) -> anyhow::Result<TestResult<Vec<Event>>> {
        self.execute_messages(signer, gas_limit, vec![msg])
    }

    pub fn execute_messages(
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
        self.block.timestamp = self
            .block
            .timestamp
            .plus_nanos(self.block_time.as_nanos() as u64);

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

    pub fn deploy_contract<M>(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        code: Binary,
        salt: Binary,
        msg: &M,
    ) -> anyhow::Result<Addr>
    where
        M: Serialize,
    {
        let code_hash = Hash::from_slice(sha2_256(&code));
        let address = Addr::compute(&signer.address, &code_hash, &salt);

        self.execute_messages(signer, gas_limit, vec![
            Message::Upload { code },
            Message::Instantiate {
                code_hash,
                msg: to_json_value(&msg)?,
                salt: salt.to_vec().into(),
                funds: Coins::new_empty(),
                admin: None,
            },
        ])?
        .should_succeed()?;

        Ok(address)
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
}
