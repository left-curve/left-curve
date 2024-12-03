use {
    crate::{
        account::multi::ParamUpdates,
        account_factory::{Account, AccountIndex, AccountParams, AccountType, Username},
        auth::Key,
    },
    grug::{Addr, Coins, Hash160, Hash256},
    std::collections::BTreeMap,
};

/// Information about a user. Used in query response.
#[grug::derive(Serde)]
pub struct User {
    /// Keys associated with this user, indexes by hashes.
    pub keys: BTreeMap<Hash160, Key>,
    /// Accounts associated with this user, indexes by addresses.
    pub accounts: BTreeMap<Addr, Account>,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Code hash to be associated with each account type.
    pub code_hashes: BTreeMap<AccountType, Hash256>,
    /// Users with associated key to set up during genesis.
    /// Each genesis user is to be associated with exactly one key.
    /// A spot account will be created for each genesis user.
    pub users: BTreeMap<Username, (Hash160, Key)>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Make an initial deposit, prior to registering a username.
    ///
    /// This the first of the two-step user onboarding process.
    ///
    /// This method can only be invoked by the IBC transfer contract.
    Deposit { recipient: Addr },
    /// Create a new user, following an initial deposit.
    ///
    /// This is the second of the two-step user onboarding process.
    RegisterUser {
        username: Username,
        key: Key,
        key_hash: Hash160,
    },
    /// Register a new account for an existing user.
    RegisterAccount { params: AccountParams },
    /// Update a Safe account's parameters.
    ConfigureSafe { updates: ParamUpdates },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the account index, which is used in deriving the account address,
    /// that will be used if a user is to create a new account.
    #[returns(AccountIndex)]
    NextAccountIndex {},
    /// Query the code hash associated with the an account type.
    #[returns(Hash256)]
    CodeHash { account_type: AccountType },
    /// Enumerate all code hashes associated with account types.
    #[returns(BTreeMap<AccountType, Hash256>)]
    CodeHashes {
        start_after: Option<AccountType>,
        limit: Option<u32>,
    },
    /// Query unclaimed deposit for the given address.
    #[returns(Coins)]
    Deposit { recipient: Addr },
    /// Enumerate all unclaimed deposits.
    #[returns(BTreeMap<Addr, Coins>)]
    Deposits {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    /// Query a key by its hash associated to a username.
    #[returns(Key)]
    Key { hash: Hash160, username: Username },
    /// Enumerate all keys.
    #[returns(BTreeMap<(Username, Hash160), Key>)]
    Keys {
        start_after: Option<(Username, Hash160)>,
        limit: Option<u32>,
    },
    /// Find all keys associated with a user.
    #[returns(BTreeMap<Hash160, Key>)]
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
