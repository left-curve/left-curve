use {
    anyhow::{anyhow, bail, ensure},
    grug_account::{make_sign_bytes, PublicKey, StateResponse},
    grug_app::{App, AppResult},
    grug_crypto::{sha2_256, Identity256},
    grug_db_memory::MemDb,
    grug_types::{
        from_json_value, to_json_value, Addr, Binary, BlockInfo, Coin, Coins, Config, Empty, Event,
        GenesisState, Hash, Message, NumberConst, Permission, Permissions, QueryRequest, Timestamp,
        Tx, Uint128, Uint64, GENESIS_SENDER,
    },
    grug_vm_wasm::{VmError, WasmVm},
    k256::ecdsa::{signature::DigestSigner, Signature, SigningKey},
    rand::rngs::OsRng,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{
        collections::{BTreeMap, BTreeSet},
        fmt::Debug,
        fs, io,
        sync::Once,
        vec,
    },
};

const MOCK_CHAIN_ID: &str = "grug-1";
const MOCK_BLOCK_TIME_NANOS: u64 = 250_000_000; // 250 milliseconds
const MOCK_DENOM: &str = "ugrug";
const MOCK_BANK_SALT: &[u8] = b"bank";
const MOCK_SENDER_SALT: &[u8] = b"sender";
const MOCK_RECEIVER_SALT: &[u8] = b"receiver";

// The tracing subscriber can only be set once. We ensure this by using `Once`.
static TRACING: Once = Once::new();

fn setup_tracing() {
    TRACING.call_once(|| {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global tracing subscriber");
    });
}

fn read_wasm_file(filename: &str) -> io::Result<Vec<u8>> {
    let path = format!("{}/testdata/{filename}", env!("CARGO_MANIFEST_DIR"));
    fs::read(path)
}

struct TestAccount {
    address: Addr,
    sk: SigningKey,
    pk: Binary,
}

impl TestAccount {
    fn new_random(code_hash: &Hash, salt: &[u8]) -> anyhow::Result<Self> {
        let address = Addr::compute(&GENESIS_SENDER, code_hash, salt);
        let sk = SigningKey::random(&mut OsRng);
        let pk = sk
            .verifying_key()
            .to_encoded_point(true)
            .to_bytes()
            .to_vec()
            .into();

        Ok(Self { address, sk, pk })
    }

    fn sign_transaction(
        &self,
        suite: &TestSuite,
        msgs: Vec<Message>,
        gas_limit: u64,
    ) -> anyhow::Result<Tx> {
        // Query the account's sequence.
        // This assumes the account is the default "grug-account" contract.
        let sequence = suite
            .query_wasm_smart::<_, StateResponse>(
                self.address.clone(),
                &grug_account::QueryMsg::State {},
            )
            .should_succeed()?
            .sequence;

        // Sign the transaction
        let sign_bytes = Identity256::from(make_sign_bytes(
            sha2_256,
            &msgs,
            &self.address,
            MOCK_CHAIN_ID,
            sequence,
        )?);
        let signature: Signature = self.sk.sign_digest(sign_bytes);

        Ok(Tx {
            sender: self.address.clone(),
            msgs,
            gas_limit,
            credential: signature.to_vec().into(),
        })
    }
}

struct TestSuite {
    app: App<MemDb, WasmVm>,
    block: BlockInfo,
}

impl TestSuite {
    /// Initialize the chain with the default genesis state for these tests.
    /// This includes two user accounts and the bank contract.
    fn default_setup() -> anyhow::Result<(Self, TestAccount, TestAccount)> {
        let app = App::new(MemDb::new(), WasmVm::new(100), None);
        let block = BlockInfo {
            height: Uint64::ZERO,
            timestamp: Timestamp::from_nanos(0),
            hash: Hash::ZERO,
        };

        // Load account contract byte code, and generate two test accounts.
        let account_code = read_wasm_file("grug_account.wasm")?;
        let account_code_hash = Hash::from_slice(sha2_256(&account_code));
        let sender = TestAccount::new_random(&account_code_hash, MOCK_SENDER_SALT)?;
        let receiver = TestAccount::new_random(&account_code_hash, MOCK_RECEIVER_SALT)?;

        // Load bank contract byte code, and predict its address.
        let bank_code = read_wasm_file("grug_bank.wasm")?;
        let bank_code_hash = Hash::from_slice(sha2_256(&bank_code));
        let bank = Addr::compute(&GENESIS_SENDER, &bank_code_hash, MOCK_BANK_SALT);

        // Genesis the chain. This deploys the bank contract and gives the "sender"
        // account 100 ugrug.
        app.do_init_chain(MOCK_CHAIN_ID.to_string(), block.clone(), GenesisState {
            config: Config {
                owner: None,
                bank: bank.clone(),
                begin_blockers: vec![],
                end_blockers: vec![],
                permissions: Permissions {
                    upload: Permission::Everybody,
                    instantiate: Permission::Everybody,
                    create_client: Permission::Everybody,
                    create_connection: Permission::Everybody,
                    create_channel: Permission::Everybody,
                },
                allowed_clients: BTreeSet::new(),
            },
            msgs: vec![
                Message::Upload {
                    code: account_code.into(),
                },
                Message::Upload {
                    code: bank_code.into(),
                },
                Message::Instantiate {
                    code_hash: account_code_hash.clone(),
                    msg: to_json_value(&grug_account::InstantiateMsg {
                        public_key: PublicKey::Secp256k1(sender.pk.clone()),
                    })?,
                    salt: MOCK_SENDER_SALT.to_vec().into(),
                    funds: Coins::new_empty(),
                    admin: Some(sender.address.clone()),
                },
                Message::Instantiate {
                    code_hash: account_code_hash.clone(),
                    msg: to_json_value(&grug_account::InstantiateMsg {
                        public_key: PublicKey::Secp256k1(receiver.pk.clone()),
                    })?,
                    salt: MOCK_RECEIVER_SALT.to_vec().into(),
                    funds: Coins::new_empty(),
                    admin: Some(receiver.address.clone()),
                },
                Message::Instantiate {
                    code_hash: bank_code_hash,
                    msg: to_json_value(&grug_bank::InstantiateMsg {
                        initial_balances: BTreeMap::from([(
                            sender.address.clone(),
                            Coins::new_one(MOCK_DENOM, 100_u128),
                        )]),
                    })?,
                    salt: MOCK_BANK_SALT.to_vec().into(),
                    funds: Coins::new_empty(),
                    admin: None,
                },
            ],
        })?;

        Ok((Self { app, block }, sender, receiver))
    }

    /// Finalize and commit a block that contains a single transaction
    /// containing the given messages.
    fn execute_messages(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        msgs: Vec<Message>,
    ) -> anyhow::Result<TestResult<Vec<Event>>> {
        // Sign the transaction
        let tx = signer.sign_transaction(self, msgs, gas_limit)?;

        // Increment block height and block time
        self.block.height += Uint64::ONE;
        self.block.timestamp = self.block.timestamp.plus_nanos(MOCK_BLOCK_TIME_NANOS);

        // Finalize block
        // Use a zero hash to mock the transaction hash.
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

    /// Deploy a contract.
    fn deploy_contract<M>(
        &mut self,
        signer: &TestAccount,
        gas_limit: u64,
        filename: &str,
        salt: &[u8],
        msg: &M,
    ) -> anyhow::Result<Addr>
    where
        M: Serialize,
    {
        let code = read_wasm_file(filename)?;
        let code_hash = Hash::from_slice(sha2_256(&code));
        let address = Addr::compute(&signer.address, &code_hash, salt);

        self.execute_messages(signer, gas_limit, vec![
            Message::Upload { code: code.into() },
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

    fn query_wasm_smart<M: Serialize, R: DeserializeOwned>(
        &self,
        contract: Addr,
        msg: &M,
    ) -> TestResult<R> {
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

    fn query_balance(&self, account: &TestAccount, denom: &str) -> TestResult<Uint128> {
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

/// A wrapper over the `AppResult`, providing two convenience methods to make
/// our tests more readable.
struct TestResult<T> {
    inner: AppResult<T>,
}

impl<T> From<AppResult<T>> for TestResult<T> {
    fn from(inner: AppResult<T>) -> Self {
        Self { inner }
    }
}

impl<T> TestResult<T> {
    /// Ensure the result is ok; return the value.
    fn should_succeed(self) -> anyhow::Result<T> {
        self.inner
            .map_err(|err| anyhow!("expecting ok, got error: {err}"))
    }

    /// Ensure the result is error, and contains the given message.
    ///
    /// Here we stringify the error and check for the existence of the substring,
    /// instead of utilizing the Rust type system.
    ///
    /// Have to go with this approach because errors emitted by the contract are
    /// converted to strings (as `GenericResult`) when passed through the FFI,
    /// at which time they lost their types.
    fn should_fail_with_error(self, msg: impl ToString) -> anyhow::Result<()> {
        match self.inner {
            Err(err) => {
                let expect = msg.to_string();
                let actual = err.to_string();
                ensure!(
                    actual.contains(&expect),
                    "wrong error! expect: {expect}, actual: {actual}"
                );
            },
            Ok(_) => bail!("expecting error, got ok"),
        }
        Ok(())
    }
}

impl<T> TestResult<T>
where
    T: Debug,
{
    /// Ensure the result is ok, and matches the expect value.
    fn should_succeed_and_equal<V>(self, expect: V) -> anyhow::Result<()>
    where
        T: PartialEq<V>,
        V: Debug,
    {
        match self.inner {
            Ok(value) => {
                ensure!(
                    value == expect,
                    "value does not match expected! expect: {expect:?}, actual: {value:?}"
                );
            },
            Err(err) => bail!("expecting ok, got error: {err}"),
        }
        Ok(())
    }
}

#[test]
fn bank_transfers() -> anyhow::Result<()> {
    setup_tracing();

    let (mut suite, sender, receiver) = TestSuite::default_setup()?;

    // Check that sender has been given 100 ugrug.
    suite
        .query_balance(&sender, MOCK_DENOM)
        .should_succeed_and_equal(Uint128::new(100))?;

    // Sender sends 70 ugrug to the receiver across multiple messages.
    suite
        .execute_messages(&sender, 900_000, vec![
            Message::Transfer {
                to: receiver.address.clone(),
                coins: vec![Coin::new(MOCK_DENOM, 10_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: receiver.address.clone(),
                coins: vec![Coin::new(MOCK_DENOM, 15_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: receiver.address.clone(),
                coins: vec![Coin::new(MOCK_DENOM, 20_u128)].try_into().unwrap(),
            },
            Message::Transfer {
                to: receiver.address.clone(),
                coins: vec![Coin::new(MOCK_DENOM, 25_u128)].try_into().unwrap(),
            },
        ])?
        .should_succeed()?;

    // Check balances again.
    suite
        .query_balance(&sender, MOCK_DENOM)
        .should_succeed_and_equal(Uint128::new(30))?;
    suite
        .query_balance(&receiver, MOCK_DENOM)
        .should_succeed_and_equal(Uint128::new(70))?;

    Ok(())
}

#[test]
fn out_of_gas() -> anyhow::Result<()> {
    setup_tracing();

    let (mut suite, sender, receiver) = TestSuite::default_setup()?;

    // Make a bank transfer with a small gas limit; should fail.
    // Bank transfers should take around 130,000 gas.
    suite
        .execute_messages(&sender, 100_000, vec![Message::Transfer {
            to: receiver.address.clone(),
            coins: vec![Coin::new(MOCK_DENOM, 10_u128)].try_into().unwrap(),
        }])?
        .should_fail_with_error("out of gas")?;

    // Tx is went out of gas.
    // Balances should remain the same
    suite
        .query_balance(&sender, MOCK_DENOM)
        .should_succeed_and_equal(Uint128::new(100))?;
    suite
        .query_balance(&receiver, MOCK_DENOM)
        .should_succeed_and_equal(Uint128::ZERO)?;

    Ok(())
}

#[test]
fn infinite_loop() -> anyhow::Result<()> {
    setup_tracing();

    let (mut suite, sender, _) = TestSuite::default_setup()?;

    let tester = suite.deploy_contract(
        &sender,
        80_000_000,
        "grug_tester_infinite_loop.wasm",
        b"tester/infinite_loop",
        &Empty {},
    )?;

    suite
        .execute_messages(&sender, 1_000_000, vec![Message::Execute {
            contract: tester,
            msg: to_json_value(&Empty {})?,
            funds: Coins::new_empty(),
        }])?
        .should_fail_with_error("out of gas")?;

    Ok(())
}

#[test]
fn immutable_state() -> anyhow::Result<()> {
    setup_tracing();

    let (mut suite, sender, _) = TestSuite::default_setup()?;

    // Deploy the tester contract
    let tester = suite.deploy_contract(
        &sender,
        80_000_000,
        "grug_tester_immutable_state.wasm",
        b"tester/immutable_state",
        &Empty {},
    )?;

    // Query the tester contract.
    //
    // During the query, the contract attempts to write to the state by directly
    // calling the `db_write` import.
    //
    // This tests how the VM handles state mutability while serving the `Query`
    // ABCI request.
    suite
        .query_wasm_smart::<_, Empty>(tester.clone(), &Empty {})
        .should_fail_with_error(VmError::ReadOnly)?;

    // Execute the tester contract.
    //
    // During the execution, the contract makes a query to itself and the query
    // tries to write to the storage.
    //
    // This tests how the VM handles state mutability while serving the
    // `FinalizeBlock` ABCI request.
    suite
        .execute_messages(&sender, 1_000_000, vec![Message::Execute {
            contract: tester,
            msg: to_json_value(&Empty {})?,
            funds: Coins::default(),
        }])?
        .should_fail_with_error(VmError::ReadOnly)?;

    Ok(())
}
