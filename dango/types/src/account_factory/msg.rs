use {
    crate::{
        account_factory::{
            Account, AccountIndex, AccountParamUpdates, AccountParams, AccountType, NewUserSalt,
            Username,
        },
        auth::{Key, Signature},
    },
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

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Code hash to be associated with each account type.
    pub code_hashes: BTreeMap<AccountType, Hash256>,
    /// Users with associated key to set up during genesis.
    /// Each genesis user is to be associated with exactly one key.
    /// A spot account will be created for each genesis user.
    pub users: BTreeMap<Username, (Hash256, Key)>,
    /// The minimum deposit required to onboard a user.
    pub minimum_deposit: Coins,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a new user, following an initial deposit.
    ///
    /// This is the second of the two-step user onboarding process.
    RegisterUser {
        data: NewUserSalt,
        signature: Signature,
    },
    /// Register a new account for an existing user.
    RegisterAccount { params: AccountParams },
    /// Associate a new or disassociate an existing key with a username.
    UpdateKey { key_hash: Hash256, key: Op<Key> },
    /// Update an account's parameters.
    UpdateAccount(AccountParamUpdates),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the minimum deposit required to onboard a user.
    #[returns(Coins)]
    MinimumDeposit {},
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
