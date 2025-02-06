use {
    crate::{
        account::multi::ParamUpdates,
        account_factory::{Account, AccountIndex, AccountParams, AccountType, Username},
        auth::Key,
    },
    grug::{Addr, Coins, Hash256},
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Code hash to be associated with each account type.
    pub code_hashes: BTreeMap<AccountType, Hash256>,
    /// Users with associated key to set up during genesis.
    /// A spot account will be created for each genesis user.
    pub users: BTreeMap<Username, Key>,
    /// The minimum deposit required to onboard a user.
    pub minimum_deposit: Coins,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a new user, following an initial deposit.
    ///
    /// This is the second of the two-step user onboarding process.
    RegisterUser { username: Username, key: Key },
    /// Register a new account for an existing user.
    RegisterAccount { params: AccountParams },
    /// Change the key associated with a username.
    ConfigureKey {
        new_key: Key,
        // TODO: require a signature from a new key to authorize this change?
    },
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
    /// Query a key associated with a username.
    #[returns(Key)]
    Key { username: Username },
    /// Enumerate all keys associated with all usernames.
    #[returns(BTreeMap<Username, Key>)]
    Keys {
        start_after: Option<Username>,
        limit: Option<u32>,
    },
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
}
