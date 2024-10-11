use {
    crate::{tracing::setup_tracing_subscriber, TestAccount, TestAccounts, TestSuite, TestVm},
    anyhow::{anyhow, ensure},
    grug_app::AppError,
    grug_db_memory::MemDb,
    grug_math::Udec128,
    grug_types::{
        Addr, Binary, BlockInfo, Coins, Config, Defined, Denom, Duration, GenesisState, HashExt,
        Json, JsonSerExt, MaybeDefined, Message, Permission, Permissions, Salt, Timestamp,
        Undefined, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT, GENESIS_SENDER,
    },
    grug_vm_rust::RustVm,
    serde::Serialize,
    std::{
        collections::BTreeMap,
        str::FromStr,
        time::{SystemTime, UNIX_EPOCH},
    },
    tracing::Level,
};

const DEFAULT_TRACING_LEVEL: Level = Level::INFO;
const DEFAULT_CHAIN_ID: &str = "dev-1";
const DEFAULT_BLOCK_TIME: Duration = Duration::from_millis(250);
const DEFAULT_DEFAULT_GAS_LIMIT: u64 = 1_000_000;
const DEFAULT_BANK_SALT: &[u8] = b"bank";
const DEFAULT_TAXMAN_SALT: &[u8] = b"taxman";
const DEFAULT_FEE_DENOM: &str = "ugrug";
const DEFAULT_FEE_RATE: &str = "0";

// If the user wishes to use a custom code for account, bank, or taxman, they
// must provide both the binary code, as well as a function for creating the
// instantiate message.
struct CodeOption<B> {
    code: Binary,
    msg_builder: B,
}

pub struct TestBuilder<
    VM = RustVm,
    M1 = grug_mock_account::InstantiateMsg,
    M2 = grug_mock_bank::InstantiateMsg,
    M3 = grug_mock_taxman::InstantiateMsg,
    OW = Undefined<Addr>,
    TA = Undefined<TestAccounts>,
> {
    vm: VM,
    // Consensus parameters
    tracing_level: Option<Level>,
    chain_id: Option<String>,
    genesis_time: Option<Timestamp>,
    block_time: Option<Duration>,
    default_gas_limit: Option<u64>,
    // App configs
    app_configs: BTreeMap<String, Json>,
    // Owner
    owner: OW,
    // Accounts
    account_opt: CodeOption<Box<dyn Fn(grug_mock_account::PublicKey) -> M1>>,
    accounts: TA,
    // Bank
    bank_opt: CodeOption<Box<dyn FnOnce(BTreeMap<Addr, Coins>) -> M2>>,
    balances: BTreeMap<Addr, Coins>,
    // Taxman
    taxman_opt: CodeOption<Box<dyn FnOnce(Denom, Udec128) -> M3>>,
    fee_denom: Option<Denom>,
    fee_rate: Option<Udec128>,
}

// Clippy incorrectly thinks we can derive `Default` here, which we can't.
#[allow(clippy::new_without_default)]
impl TestBuilder<RustVm> {
    pub fn new() -> Self {
        Self::new_with_vm(RustVm::new())
    }
}

impl<VM> TestBuilder<VM>
where
    VM: TestVm,
{
    pub fn new_with_vm(vm: VM) -> Self {
        Self {
            account_opt: CodeOption {
                code: VM::default_account_code(),
                msg_builder: Box::new(|public_key| grug_mock_account::InstantiateMsg {
                    public_key,
                }),
            },
            bank_opt: CodeOption {
                code: VM::default_bank_code(),
                msg_builder: Box::new(|initial_balances| grug_mock_bank::InstantiateMsg {
                    initial_balances,
                }),
            },
            taxman_opt: CodeOption {
                code: VM::default_taxman_code(),
                msg_builder: Box::new(|fee_denom, fee_rate| grug_mock_taxman::InstantiateMsg {
                    config: grug_mock_taxman::Config {
                        fee_denom,
                        fee_rate,
                    },
                }),
            },
            vm,
            tracing_level: Some(DEFAULT_TRACING_LEVEL),
            chain_id: None,
            genesis_time: None,
            block_time: None,
            default_gas_limit: None,
            app_configs: BTreeMap::new(),
            owner: Undefined::default(),
            accounts: Undefined::default(),
            balances: BTreeMap::new(),
            fee_denom: None,
            fee_rate: None,
        }
    }
}

impl<VM, M1, M2, M3, OW, TA> TestBuilder<VM, M1, M2, M3, OW, TA>
where
    M1: Serialize,
    M2: Serialize,
    M3: Serialize,
    OW: MaybeDefined<Inner = Addr>,
    TA: MaybeDefined<Inner = TestAccounts>,
    VM: TestVm + Clone,
    AppError: From<VM::Error>,
{
    // Setting this to `None` means no tracing.
    pub fn set_tracing_level(mut self, level: Option<Level>) -> Self {
        self.tracing_level = level;
        self
    }

    pub fn set_chain_id<T>(mut self, chain_id: T) -> Self
    where
        T: ToString,
    {
        self.chain_id = Some(chain_id.to_string());
        self
    }

    pub fn set_genesis_time(mut self, genesis_time: Timestamp) -> Self {
        self.genesis_time = Some(genesis_time);
        self
    }

    pub fn set_block_time(mut self, block_time: Duration) -> Self {
        self.block_time = Some(block_time);
        self
    }

    pub fn set_default_gas_limit(mut self, gas_limit: u64) -> Self {
        self.default_gas_limit = Some(gas_limit);
        self
    }

    pub fn set_fee_denom(mut self, fee_denom: Denom) -> Self {
        self.fee_denom = Some(fee_denom);
        self
    }

    pub fn set_fee_rate(mut self, fee_rate: Udec128) -> Self {
        self.fee_rate = Some(fee_rate);
        self
    }

    pub fn add_app_config<K, V>(mut self, key: K, value: &V) -> anyhow::Result<Self>
    where
        K: Into<String>,
        V: Serialize,
    {
        let key = key.into();
        let value = value.to_json_value()?;

        ensure!(
            !self.app_configs.contains_key(&key),
            "app config key `{key}` is already set"
        );

        self.app_configs.insert(key, value);

        Ok(self)
    }

    /// Use a custom code for the bank instead the default implementation
    /// provided by the Grug test suite.
    ///
    /// Must provide a builder function that generates the bank's instantiate
    /// message given the initial account balances.
    ///
    /// E.g.
    ///
    /// ```rust
    /// use {
    ///     grug_testing::TestBuilder,
    ///     grug_types::Coins,
    ///     grug_vm_rust::ContractBuilder,
    /// };
    ///
    /// let code = ContractBuilder::new(Box::new(grug_mock_bank::instantiate))
    ///     .with_bank_execute(Box::new(grug_mock_bank::bank_execute))
    ///     .with_bank_query(Box::new(grug_mock_bank::bank_query))
    ///     .build();
    ///
    /// let (suite, accounts) = TestBuilder::new()
    ///     .add_account("owner", Coins::new())
    ///     .unwrap()
    ///     .set_owner("owner")
    ///     .unwrap()
    ///     .set_bank_code(
    ///         code,
    ///         |initial_balances| grug_mock_bank::InstantiateMsg { initial_balances },
    ///     )
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn set_bank_code<T, F, M2A>(
        self,
        code: T,
        msg_builder: F,
    ) -> TestBuilder<VM, M1, M2A, M3, OW, TA>
    where
        T: Into<Binary>,
        F: FnOnce(BTreeMap<Addr, Coins>) -> M2A + 'static,
    {
        TestBuilder {
            vm: self.vm,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_configs: self.app_configs,
            owner: self.owner,
            account_opt: self.account_opt,
            accounts: self.accounts,
            bank_opt: CodeOption {
                code: code.into(),
                msg_builder: Box::new(msg_builder),
            },
            balances: self.balances,
            taxman_opt: self.taxman_opt,
            fee_denom: self.fee_denom,
            fee_rate: self.fee_rate,
        }
    }

    /// Use a custom code for the taxman instead the default implementation
    /// provided by the Grug test suite.
    ///
    /// Must provide a builder function that generates the taxman's instantiate
    /// message given the fee denom and fee rate.
    ///
    /// E.g.
    ///
    /// ```rust
    /// use {
    ///     grug_testing::TestBuilder,
    ///     grug_types::Coins,
    ///     grug_vm_rust::ContractBuilder,
    /// };
    ///
    /// let code = ContractBuilder::new(Box::new(grug_mock_taxman::instantiate))
    ///     .with_withhold_fee(Box::new(grug_mock_taxman::withhold_fee))
    ///     .with_finalize_fee(Box::new(grug_mock_taxman::finalize_fee))
    ///     .build();
    ///
    /// let (suite, accounts) = TestBuilder::new()
    ///     .add_account("owner", Coins::new())
    ///     .unwrap()
    ///     .set_owner("owner")
    ///     .unwrap()
    ///     .set_taxman_code(
    ///         code,
    ///         |fee_denom, fee_rate| grug_mock_taxman::InstantiateMsg {
    ///             config: grug_mock_taxman::Config { fee_denom, fee_rate },
    ///         },
    ///     )
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn set_taxman_code<T, F, M3A>(
        self,
        code: T,
        msg_builder: F,
    ) -> TestBuilder<VM, M1, M2, M3A, OW, TA>
    where
        T: Into<Binary>,
        F: FnOnce(Denom, Udec128) -> M3A + 'static,
    {
        TestBuilder {
            vm: self.vm,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_configs: self.app_configs,
            owner: self.owner,
            account_opt: self.account_opt,
            accounts: self.accounts,
            bank_opt: self.bank_opt,
            balances: self.balances,
            taxman_opt: CodeOption {
                code: code.into(),
                msg_builder: Box::new(msg_builder),
            },
            fee_denom: self.fee_denom,
            fee_rate: self.fee_rate,
        }
    }

    pub fn add_account<C>(
        mut self,
        name: &'static str,
        balances: C,
    ) -> anyhow::Result<TestBuilder<VM, M1, M2, M3, OW, Defined<TestAccounts>>>
    where
        C: TryInto<Coins>,
        anyhow::Error: From<C::Error>,
    {
        let mut accounts = self.accounts.maybe_inner().unwrap_or_default();
        ensure!(
            !accounts.contains_key(name),
            "account with name {name} already exists"
        );

        // Generate a random new account
        let account = TestAccount::new_random(self.account_opt.code.hash256(), name.as_bytes());

        // Save account and balances
        let balances = balances.try_into()?;
        if !balances.is_empty() {
            self.balances.insert(account.address, balances);
        }
        accounts.insert(name, account);

        Ok(TestBuilder {
            vm: self.vm,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_configs: self.app_configs,
            owner: self.owner,
            account_opt: self.account_opt,
            accounts: Defined::new(accounts),
            bank_opt: self.bank_opt,
            balances: self.balances,
            taxman_opt: self.taxman_opt,
            fee_denom: self.fee_denom,
            fee_rate: self.fee_rate,
        })
    }
}

impl<VM, M1, M2, M3, OW> TestBuilder<VM, M1, M2, M3, OW, Undefined<TestAccounts>>
where
    M1: Serialize,
    M2: Serialize,
    M3: Serialize,
    OW: MaybeDefined<Inner = Addr>,
    VM: TestVm + Clone,
    AppError: From<VM::Error>,
{
    /// Use a custom code for the account instead the default implementation
    /// provided by the Grug test suite.
    ///
    /// Must provide a builder function that generates an account's instantiate
    /// message given an Secp256k1 public key.
    ///
    /// **Note:** `set_account_code` can only be called before any `add_account`
    /// call, otherwise the derived addresses won't be correct.
    ///
    /// E.g.
    ///
    /// ```rust
    /// use {
    ///     grug_testing::TestBuilder,
    ///     grug_types::Coins,
    ///     grug_vm_rust::ContractBuilder,
    /// };
    ///
    /// let code = ContractBuilder::new(Box::new(grug_mock_account::instantiate))
    ///     .with_authenticate(Box::new(grug_mock_account::authenticate))
    ///     .build();
    ///
    /// let (suite, accounts) = TestBuilder::new()
    ///     .set_account_code(
    ///         code,
    ///         |public_key| grug_mock_account::InstantiateMsg { public_key },
    ///     )
    ///     .unwrap()
    ///     .add_account("owner", Coins::new())
    ///     .unwrap()
    ///     .set_owner("owner")
    ///     .unwrap()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn set_account_code<T, F, M1A>(
        self,
        code: T,
        msg_builder: F,
    ) -> anyhow::Result<TestBuilder<VM, M1A, M2, M3, OW, Undefined<TestAccounts>>>
    where
        T: Into<Binary>,
        F: Fn(grug_mock_account::PublicKey) -> M1A + 'static,
    {
        Ok(TestBuilder {
            vm: self.vm,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_configs: self.app_configs,
            owner: self.owner,
            account_opt: CodeOption {
                code: code.into(),
                msg_builder: Box::new(msg_builder),
            },
            accounts: self.accounts,
            bank_opt: self.bank_opt,
            balances: self.balances,
            taxman_opt: self.taxman_opt,
            fee_denom: self.fee_denom,
            fee_rate: self.fee_rate,
        })
    }
}

// `set_owner` can only be called if `add_accounts` has been called at least
// once, and `set_owner` hasn't already been called.
impl<VM, M1, M2, M3> TestBuilder<VM, M1, M2, M3, Undefined<Addr>, Defined<TestAccounts>> {
    pub fn set_owner(
        self,
        name: &'static str,
    ) -> anyhow::Result<TestBuilder<VM, M1, M2, M3, Defined<Addr>, Defined<TestAccounts>>> {
        let owner =
            self.accounts.inner().get(name).ok_or_else(|| {
                anyhow!("failed to set owner: can't find account with name `{name}`")
            })?;

        Ok(TestBuilder {
            vm: self.vm,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_configs: self.app_configs,
            owner: Defined::new(owner.address),
            account_opt: self.account_opt,
            accounts: self.accounts,
            bank_opt: self.bank_opt,
            balances: self.balances,
            taxman_opt: self.taxman_opt,
            fee_denom: self.fee_denom,
            fee_rate: self.fee_rate,
        })
    }
}

// `build` can only be called if both `owner` and `accounts` have been set.
impl<VM, M1, M2, M3> TestBuilder<VM, M1, M2, M3, Defined<Addr>, Defined<TestAccounts>>
where
    M1: Serialize,
    M2: Serialize,
    M3: Serialize,
    VM: TestVm + Clone,
    AppError: From<VM::Error>,
{
    pub fn build(self) -> anyhow::Result<(TestSuite<MemDb, VM>, TestAccounts)> {
        if let Some(tracing_level) = self.tracing_level {
            setup_tracing_subscriber(tracing_level);
        }

        let block_time = self.block_time.unwrap_or(DEFAULT_BLOCK_TIME);

        let default_gas_limit = self.default_gas_limit.unwrap_or(DEFAULT_DEFAULT_GAS_LIMIT);

        let chain_id = self
            .chain_id
            .unwrap_or_else(|| DEFAULT_CHAIN_ID.to_string());

        let fee_denom = self
            .fee_denom
            .unwrap_or_else(|| Denom::from_str(DEFAULT_FEE_DENOM).unwrap());

        let fee_rate = self
            .fee_rate
            .unwrap_or_else(|| Udec128::from_str(DEFAULT_FEE_RATE).unwrap());

        // Use the current system time as genesis time, if unspecified.
        let genesis_time = match self.genesis_time {
            Some(time) => time,
            None => Timestamp::from_nanos(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()),
        };

        let genesis_block = BlockInfo {
            hash: GENESIS_BLOCK_HASH,
            height: GENESIS_BLOCK_HEIGHT,
            timestamp: genesis_time,
        };

        // Upload account, bank, and taxman codes,
        // instantiate bank and taxman contracts.
        let mut msgs = vec![
            Message::upload(self.account_opt.code.clone()),
            Message::upload(self.bank_opt.code.clone()),
            Message::upload(self.taxman_opt.code.clone()),
            Message::instantiate(
                self.bank_opt.code.hash256(),
                &(self.bank_opt.msg_builder)(self.balances),
                Salt::new(DEFAULT_BANK_SALT.into())?,
                Coins::new(),
                None,
            )?,
            Message::instantiate(
                self.taxman_opt.code.hash256(),
                &(self.taxman_opt.msg_builder)(fee_denom, fee_rate),
                Salt::new(DEFAULT_TAXMAN_SALT.into())?,
                Coins::new(),
                None,
            )?,
        ];

        // Instantiate accounts
        for (name, account) in self.accounts.inner() {
            msgs.push(Message::instantiate(
                self.account_opt.code.hash256(),
                &(self.account_opt.msg_builder)(account.pk),
                Salt::from_str(name)?,
                Coins::new(),
                Some(account.address),
            )?);
        }

        // Predict bank contract address
        let bank = Addr::compute(
            GENESIS_SENDER,
            self.bank_opt.code.hash256(),
            DEFAULT_BANK_SALT,
        );

        // Prefict taxman contract address
        let taxman = Addr::compute(
            GENESIS_SENDER,
            self.taxman_opt.code.hash256(),
            DEFAULT_TAXMAN_SALT,
        );

        // Create the app config
        let config = Config {
            owner: self.owner.into_inner(),
            bank,
            taxman,
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Everybody,
                instantiate: Permission::Everybody,
            },
        };

        let genesis_state = GenesisState {
            config,
            msgs,
            app_configs: self.app_configs,
        };

        let suite = TestSuite::new_with_vm(
            self.vm,
            chain_id,
            block_time,
            default_gas_limit,
            genesis_block,
            genesis_state,
        )?;

        Ok((suite, self.accounts.into_inner()))
    }
}
