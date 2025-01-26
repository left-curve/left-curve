use {
    crate::{
        account::multi::ParamUpdates,
        account_factory::{Account, AccountIndex, AccountParams, AccountType, Username},
        auth::Key,
    },
    anyhow::anyhow,
    grug::{Addr, Coins, Hash256, Op},
    std::collections::BTreeMap,
};

/// Information about a user. Used in query response.
#[grug::derive(Serde)]
pub struct User {
    /// Keys associated with this user, indexes by hashes.
    pub keys: BTreeMap<Hash256, Key>,
    /// Accounts associated with this user, indexes by addresses.
    pub accounts: BTreeMap<Addr, Account>,
}

#[grug::derive(Serde, Borsh)]
pub struct Config {
    /// The minimum amount of deposit required to register a new user.
    pub minimum_deposits: Coins,
    /// Code hash to be associated with each account type.
    pub code_hashes: BTreeMap<AccountType, Hash256>,
}

impl Config {
    pub fn code_hash_for(&self, ty: AccountType) -> anyhow::Result<Hash256> {
        self.code_hashes
            .get(&ty)
            .copied()
            .ok_or_else(|| anyhow!("code hash not found for account type: {ty}"))
    }
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
    /// Users to be registered during genesis.
    ///
    ///
    /// Note:
    ///
    /// - Each genesis user is to be associated with exactly one key.
    /// - A spot account will be created for each genesis user.
    /// - Genesis users don't need to make initial deposits.
    pub users: BTreeMap<Username, (Hash256, Key)>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a new user with an initial deposit.
    RegisterUser {
        username: Username,
        key: Key,
        key_hash: Hash256,
    },
    /// Register a new account for an existing user.
    RegisterAccount { params: AccountParams },
    /// Configure a key for a username.
    ConfigureKey { key_hash: Hash256, key: Op<Key> },
    /// Update a Safe account's parameters.
    ConfigureSafe { updates: ParamUpdates },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Return the account factory configuration.
    #[returns(Config)]
    Config {},
    /// Query the account index, which is used in deriving the account address,
    /// that will be used if a user is to create a new account.
    #[returns(AccountIndex)]
    NextAccountIndex {},
    /// Query a key by its hash associated to a username.
    #[returns(Key)]
    Key { hash: Hash256, username: Username },
    /// Enumerate all keys.
    #[returns(Vec<QueryKeyResponseItem>)]
    Keys {
        start_after: Option<QueryKeyPaginateParam>,
        limit: Option<u32>,
    },
    /// Find all keys associated with a user.
    #[returns(BTreeMap<Hash256, Key>)]
    KeysByUser { username: Username },
    /// Query parameters of an account by address.
    #[returns(Account)]
    Account { address: Addr },
    /// Enumerate all accounts and addresses.
    #[returns(BTreeMap<Addr, Account>)]
    Accounts {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    /// Find all accounts associated with a user.
    #[returns(BTreeMap<Addr, Account>)]
    AccountsByUser { username: Username },
    /// Query a single user by username.
    #[returns(User)]
    User { username: Username },
}

#[grug::derive(Serde)]
pub struct QueryKeyPaginateParam {
    pub username: Username,
    pub key_hash: Hash256,
}

#[grug::derive(Serde)]
pub struct QueryKeyResponseItem {
    pub username: Username,
    pub key_hash: Hash256,
    pub key: Key,
}
