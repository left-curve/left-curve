use {
    crate::{TestAccount, TestAccounts, TestSuite, TestVm, tracing::setup_tracing_subscriber},
    grug_app::{AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer, ProposalPreparer},
    grug_db_memory::MemDb,
    grug_math::Udec128,
    grug_types::{
        Addr, Binary, BlockInfo, Coins, Config, Defined, Denom, Duration, GENESIS_BLOCK_HASH,
        GENESIS_BLOCK_HEIGHT, GENESIS_SENDER, GenesisState, HashExt, Json, JsonSerExt,
        MaybeDefined, Message, Permission, Permissions, StdResult, Timestamp, Undefined,
    },
    grug_vm_rust::RustVm,
    serde::Serialize,
    std::{
        collections::BTreeMap,
        fmt::Debug,
        ops::Deref,
        str::FromStr,
        time::{SystemTime, UNIX_EPOCH},
    },
    tracing::Level,
};

pub const DEFAULT_TRACING_LEVEL: Level = Level::ERROR;
pub const DEFAULT_CHAIN_ID: &str = "dev-1";
pub const DEFAULT_BLOCK_TIME: Duration = Duration::from_millis(250);
pub const DEFAULT_DEFAULT_GAS_LIMIT: u64 = 1_000_000;
pub const DEFAULT_BANK_SALT: &str = "bank";
pub const DEFAULT_TAXMAN_SALT: &str = "taxman";
pub const DEFAULT_FEE_DENOM: &str = "ugrug";
pub const DEFAULT_FEE_RATE: &str = "0";
pub const DEFAULT_MAX_ORPHAN_AGE: Duration = Duration::from_seconds(7 * 24 * 60 * 60); // 7 days

// If the user wishes to use a custom code for account, bank, or taxman, they
// must provide both the binary code, as well as a function for creating the
// instantiate message.
struct CodeOption<B> {
    code: Binary,
    msg_builder: B,
}

pub struct TestBuilder<
    DB = MemDb,
    VM = RustVm,
    PP = NaiveProposalPreparer,
    ID = NullIndexer,
    M1 = grug_mock_account::InstantiateMsg,
    M2 = grug_mock_bank::InstantiateMsg,
    M3 = grug_mock_taxman::InstantiateMsg,
    OW = Undefined<Addr>,
    TA = Undefined<TestAccounts>,
> {
    db: DB,
    vm: VM,
    pp: PP,
    indexer: ID,
    // Consensus parameters
    tracing_level: Option<Level>,
    chain_id: Option<String>,
    genesis_time: Option<Timestamp>,
    block_time: Option<Duration>,
    default_gas_limit: Option<u64>,
    // App config
    app_config: Json,
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
    max_orphan_age: Option<Duration>,
}

// Clippy incorrectly thinks we can derive `Default` here, which we can't.
#[allow(clippy::new_without_default)]
impl TestBuilder<MemDb, RustVm, NaiveProposalPreparer, NullIndexer> {
    pub fn new() -> Self {
        Self::new_with_vm_and_pp_and_indexer(
            MemDb::new(),
            RustVm::new(),
            NaiveProposalPreparer,
            NullIndexer,
        )
    }
}

impl<VM> TestBuilder<MemDb, VM>
where
    VM: TestVm,
{
    pub fn new_with_vm(vm: VM) -> Self {
        Self::new_with_vm_and_pp_and_indexer(MemDb::new(), vm, NaiveProposalPreparer, NullIndexer)
    }
}

impl<ID> TestBuilder<MemDb, RustVm, NaiveProposalPreparer, ID> {
    pub fn new_with_indexer(indexer: ID) -> Self
    where
        ID: Indexer,
    {
        Self::new_with_vm_and_pp_and_indexer(
            MemDb::new(),
            RustVm::new(),
            NaiveProposalPreparer,
            indexer,
        )
    }
}

impl<PP> TestBuilder<MemDb, RustVm, PP> {
    pub fn new_with_pp(pp: PP) -> Self {
        Self::new_with_vm_and_pp_and_indexer(MemDb::new(), RustVm::new(), pp, NullIndexer)
    }
}

impl<DB, VM, PP, ID> TestBuilder<DB, VM, PP, ID>
where
    DB: Db,
    VM: TestVm,
    ID: Indexer,
{
    pub fn new_with_vm_and_pp_and_indexer(db: DB, vm: VM, pp: PP, indexer: ID) -> Self {
        Self {
            db,
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
            pp,
            indexer,
            tracing_level: Some(DEFAULT_TRACING_LEVEL),
            chain_id: None,
            genesis_time: None,
            block_time: None,
            default_gas_limit: None,
            app_config: Json::null(),
            owner: Undefined::new(),
            accounts: Undefined::new(),
            balances: BTreeMap::new(),
            fee_denom: None,
            fee_rate: None,
            max_orphan_age: None,
        }
    }
}

impl<DB, VM, PP, ID, M1, M2, M3, OW, TA> TestBuilder<DB, VM, PP, ID, M1, M2, M3, OW, TA>
where
    DB: Db,
    ID: Indexer,
    M1: Serialize,
    M2: Serialize,
    M3: Serialize,
    OW: MaybeDefined<Addr>,
    TA: MaybeDefined<TestAccounts>,
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

    pub fn set_fee_denom<D>(mut self, fee_denom: D) -> Self
    where
        D: TryInto<Denom>,
        D::Error: Debug,
    {
        self.fee_denom = Some(fee_denom.try_into().unwrap());
        self
    }

    pub fn set_max_orphan_age(mut self, max_orphan_age: Duration) -> Self {
        self.max_orphan_age = Some(max_orphan_age);
        self
    }

    pub fn set_fee_rate(mut self, fee_rate: Udec128) -> Self {
        self.fee_rate = Some(fee_rate);
        self
    }

    pub fn set_app_config<T>(mut self, app_cfg: &T) -> StdResult<Self>
    where
        T: Serialize,
    {
        self.app_config = app_cfg.to_json_value()?;
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
    ///     .set_owner("owner")
    ///     .set_bank_code(
    ///         code,
    ///         |initial_balances| grug_mock_bank::InstantiateMsg { initial_balances },
    ///     )
    ///     .build();
    /// ```
    pub fn set_bank_code<T, F, M2A>(
        self,
        code: T,
        msg_builder: F,
    ) -> TestBuilder<DB, VM, PP, ID, M1, M2A, M3, OW, TA>
    where
        T: Into<Binary>,
        F: FnOnce(BTreeMap<Addr, Coins>) -> M2A + 'static,
    {
        TestBuilder {
            db: self.db,
            vm: self.vm,
            pp: self.pp,
            indexer: self.indexer,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_config: self.app_config,
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
            max_orphan_age: self.max_orphan_age,
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
    ///     .set_owner("owner")
    ///     .set_taxman_code(
    ///         code,
    ///         |fee_denom, fee_rate| grug_mock_taxman::InstantiateMsg {
    ///             config: grug_mock_taxman::Config { fee_denom, fee_rate },
    ///         },
    ///     )
    ///     .build();
    /// ```
    pub fn set_taxman_code<T, F, M3A>(
        self,
        code: T,
        msg_builder: F,
    ) -> TestBuilder<DB, VM, PP, ID, M1, M2, M3A, OW, TA>
    where
        T: Into<Binary>,
        F: FnOnce(Denom, Udec128) -> M3A + 'static,
    {
        TestBuilder {
            db: self.db,
            vm: self.vm,
            pp: self.pp,
            indexer: self.indexer,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_config: self.app_config,
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
            max_orphan_age: self.max_orphan_age,
        }
    }

    pub fn add_account<C>(
        mut self,
        name: &'static str,
        balances: C,
    ) -> TestBuilder<DB, VM, PP, ID, M1, M2, M3, OW, Defined<TestAccounts>>
    where
        C: TryInto<Coins>,
        C::Error: Debug,
    {
        let mut accounts = self.accounts.maybe_into_inner().unwrap_or_default();
        assert!(
            !accounts.contains_key(name),
            "account with name {name} already exists"
        );

        // Generate a random new account
        let account = TestAccount::new_random(self.account_opt.code.hash256(), name.as_bytes());

        // Save account and balances
        let balances = balances.try_into().unwrap();
        if !balances.is_empty() {
            self.balances.insert(account.address, balances);
        }
        accounts.insert(name, account);

        TestBuilder {
            db: self.db,
            vm: self.vm,
            pp: self.pp,
            indexer: self.indexer,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_config: self.app_config,
            owner: self.owner,
            account_opt: self.account_opt,
            accounts: Defined::new(accounts),
            bank_opt: self.bank_opt,
            balances: self.balances,
            taxman_opt: self.taxman_opt,
            fee_denom: self.fee_denom,
            fee_rate: self.fee_rate,
            max_orphan_age: self.max_orphan_age,
        }
    }
}

impl<DB, VM, PP, ID, M1, M2, M3, OW>
    TestBuilder<DB, VM, PP, ID, M1, M2, M3, OW, Undefined<TestAccounts>>
where
    DB: Db,
    ID: Indexer,
    M1: Serialize,
    M2: Serialize,
    M3: Serialize,
    OW: MaybeDefined<Addr>,
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
    ///     .add_account("owner", Coins::new())
    ///     .set_owner("owner")
    ///     .build();
    /// ```
    pub fn set_account_code<T, F, M1A>(
        self,
        code: T,
        msg_builder: F,
    ) -> TestBuilder<DB, VM, PP, ID, M1A, M2, M3, OW, Undefined<TestAccounts>>
    where
        T: Into<Binary>,
        F: Fn(grug_mock_account::PublicKey) -> M1A + 'static,
    {
        TestBuilder {
            db: self.db,
            vm: self.vm,
            pp: self.pp,
            indexer: self.indexer,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_config: self.app_config,
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
            max_orphan_age: self.max_orphan_age,
        }
    }
}

// `set_owner` can only be called if `add_accounts` has been called at least
// once, and `set_owner` hasn't already been called.
impl<DB, VM, PP, ID, M1, M2, M3>
    TestBuilder<DB, VM, PP, ID, M1, M2, M3, Undefined<Addr>, Defined<TestAccounts>>
{
    pub fn set_owner(
        self,
        name: &'static str,
    ) -> TestBuilder<DB, VM, PP, ID, M1, M2, M3, Defined<Addr>, Defined<TestAccounts>> {
        let owner = self.accounts.inner().get(name).unwrap_or_else(|| {
            panic!("failed to set owner: can't find account with name `{name}`")
        });

        TestBuilder {
            db: self.db,
            vm: self.vm,
            pp: self.pp,
            indexer: self.indexer,
            tracing_level: self.tracing_level,
            chain_id: self.chain_id,
            genesis_time: self.genesis_time,
            block_time: self.block_time,
            default_gas_limit: self.default_gas_limit,
            app_config: self.app_config,
            owner: Defined::new(owner.address),
            account_opt: self.account_opt,
            accounts: self.accounts,
            bank_opt: self.bank_opt,
            balances: self.balances,
            taxman_opt: self.taxman_opt,
            fee_denom: self.fee_denom,
            fee_rate: self.fee_rate,
            max_orphan_age: self.max_orphan_age,
        }
    }
}

// `build` can only be called if both `owner` and `accounts` have been set.
impl<DB, VM, PP, ID, M1, M2, M3>
    TestBuilder<DB, VM, PP, ID, M1, M2, M3, Defined<Addr>, Defined<TestAccounts>>
where
    DB: Db,
    M1: Serialize,
    M2: Serialize,
    M3: Serialize,
    VM: TestVm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<VM::Error> + From<PP::Error> + From<ID::Error> + From<DB::Error>,
{
    pub fn build(self) -> (TestSuite<DB, VM, PP, ID>, TestAccounts) {
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
        let genesis_time = self.genesis_time.unwrap_or_else(|| {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Timestamp::from_nanos(nanos)
        });

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
                DEFAULT_BANK_SALT,
                Some(DEFAULT_BANK_SALT),
                None,
                Coins::new(),
            )
            .unwrap(),
            Message::instantiate(
                self.taxman_opt.code.hash256(),
                &(self.taxman_opt.msg_builder)(fee_denom, fee_rate),
                DEFAULT_TAXMAN_SALT,
                Some(DEFAULT_TAXMAN_SALT),
                None,
                Coins::new(),
            )
            .unwrap(),
        ];

        // Instantiate accounts
        for (name, account) in self.accounts.inner().deref() {
            msgs.push(
                Message::instantiate(
                    self.account_opt.code.hash256(),
                    &(self.account_opt.msg_builder)(account.pk),
                    *name,
                    Some(format!("account/{name}")),
                    Some(account.address),
                    Coins::new(),
                )
                .unwrap(),
            );
        }

        // Predict bank contract address
        let bank = Addr::derive(
            GENESIS_SENDER,
            self.bank_opt.code.hash256(),
            DEFAULT_BANK_SALT.as_bytes(),
        );

        // Prefict taxman contract address
        let taxman = Addr::derive(
            GENESIS_SENDER,
            self.taxman_opt.code.hash256(),
            DEFAULT_TAXMAN_SALT.as_bytes(),
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
            max_orphan_age: self.max_orphan_age.unwrap_or(DEFAULT_MAX_ORPHAN_AGE),
        };

        let genesis_state = GenesisState {
            config,
            msgs,
            app_config: self.app_config,
        };

        let suite = TestSuite::new_with_db_vm_indexer_and_pp(
            self.db,
            self.vm,
            self.pp,
            self.indexer,
            chain_id,
            block_time,
            default_gas_limit,
            genesis_block,
            genesis_state,
        );

        (suite, self.accounts.into_inner())
    }
}
