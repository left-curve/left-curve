use {
    crate::{tracing::setup_tracing_subscriber, TestAccount, TestAccounts, TestSuite, TestVm},
    anyhow::{anyhow, ensure},
    grug_account::PublicKey,
    grug_app::AppError,
    grug_types::{
        hash, Addr, Binary, BlockInfo, Coins, Config, Duration, GenesisState, Hash, Message,
        Permission, Permissions, Timestamp, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT,
        GENESIS_SENDER,
    },
    grug_vm_rust::RustVm,
    std::{
        collections::BTreeMap,
        time::{SystemTime, UNIX_EPOCH},
    },
    tracing::Level,
};

const DEFAULT_TRACING_LEVEL: Level = Level::DEBUG;
const DEFAULT_CHAIN_ID: &str = "dev-1";
const DEFAULT_BLOCK_TIME: Duration = Duration::from_millis(250);
const DEFAULT_BANK_SALT: &[u8] = b"bank";

pub struct TestBuilder<VM = RustVm>
where
    VM: TestVm,
{
    vm: VM,
    tracing_level: Option<Level>,
    chain_id: Option<String>,
    genesis_time: Option<Timestamp>,
    block_time: Option<Duration>,
    owner: Option<Addr>,
    // TODO: let user customize the codes and instantiate messages of bank and account
    account_code: Binary,
    account_code_hash: Hash,
    accounts: TestAccounts,
    bank_code: Binary,
    bank_code_hash: Hash,
    balances: BTreeMap<Addr, Coins>,
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
    VM: TestVm + Clone,
    AppError: From<VM::Error>,
{
    pub fn new_with_vm(vm: VM) -> Self {
        let account_code = VM::default_account_code();
        let account_code_hash = hash(&account_code);

        let bank_code = VM::default_bank_code();
        let bank_code_hash = hash(&bank_code);

        Self {
            vm,
            tracing_level: Some(DEFAULT_TRACING_LEVEL),
            chain_id: None,
            genesis_time: None,
            block_time: None,
            owner: None,
            account_code,
            account_code_hash,
            accounts: TestAccounts::new(),
            bank_code,
            bank_code_hash,
            balances: BTreeMap::new(),
        }
    }

    // Setting this to `None` means no tracing.
    pub fn set_tracing_level(mut self, level: Option<Level>) -> Self {
        self.tracing_level = level;
        self
    }

    pub fn set_chain_id(mut self, chain_id: impl ToString) -> Self {
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

    pub fn set_owner(mut self, name: &'static str) -> anyhow::Result<Self> {
        let owner = self
            .accounts
            .get(name)
            .ok_or_else(|| anyhow!("failed to set owner: can't find account with name `{name}`"))?;

        self.owner = Some(owner.address.clone());

        Ok(self)
    }

    pub fn add_account<C>(mut self, name: &'static str, balances: C) -> anyhow::Result<Self>
    where
        C: TryInto<Coins>,
        anyhow::Error: From<C::Error>,
    {
        ensure!(
            !self.accounts.contains_key(name),
            "account with name {name} already exists"
        );

        // Generate a random new account
        let account = TestAccount::new_random(&self.account_code_hash, name.as_bytes());

        // Save account and balances
        let balances = balances.try_into()?;
        if !balances.is_empty() {
            self.balances.insert(account.address.clone(), balances);
        }
        self.accounts.insert(name, account);

        Ok(self)
    }

    pub fn build(self) -> anyhow::Result<(TestSuite<VM>, TestAccounts)> {
        if let Some(tracing_level) = self.tracing_level {
            setup_tracing_subscriber(tracing_level);
        }

        let block_time = self.block_time.unwrap_or(DEFAULT_BLOCK_TIME);

        let chain_id = self
            .chain_id
            .unwrap_or_else(|| DEFAULT_CHAIN_ID.to_string());

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

        // Upload account and bank codes, instantiate bank contract.
        let mut msgs = vec![
            Message::upload(self.account_code),
            Message::upload(self.bank_code),
            Message::instantiate(
                self.bank_code_hash.clone(),
                &grug_bank::InstantiateMsg {
                    initial_balances: self.balances,
                },
                DEFAULT_BANK_SALT,
                Coins::new(),
                None,
            )?,
        ];

        // Instantiate accounts
        for (name, account) in &self.accounts {
            msgs.push(Message::instantiate(
                self.account_code_hash.clone(),
                &grug_account::InstantiateMsg {
                    public_key: PublicKey::Secp256k1(account.pk.clone()),
                },
                name.to_string(),
                Coins::new(),
                Some(account.address.clone()),
            )?);
        }

        // Create the app config
        let bank = Addr::compute(&GENESIS_SENDER, &self.bank_code_hash, DEFAULT_BANK_SALT);
        let config = Config {
            owner: self.owner,
            bank,
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Everybody,
                instantiate: Permission::Everybody,
            },
        };

        let genesis_state = GenesisState { config, msgs };
        let suite = TestSuite::create(self.vm, chain_id, block_time, genesis_block, genesis_state)?;

        Ok((suite, self.accounts))
    }
}
