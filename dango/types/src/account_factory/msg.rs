use {
    crate::{
        account_factory::{
            Account, AccountIndex, AccountParamUpdates, AccountParams, AccountType, UserIndex,
            Username,
        },
        auth::{Key, Signature},
    },
    grug::{Addr, Coins, Hash256, JsonSerExt, Op, SignData, StdError, StdResult},
    sha2::Sha256,
    std::collections::BTreeMap,
};

/// Option for querying a user. A user can be queried by either its numerical
/// index or its human-readable username.
#[grug::derive(Serde)]
pub enum UserQuery {
    ByName(Username),
    ByIndex(UserIndex),
}

/// Identifiers of a user. Includes a numerical identifier (used by smart
/// contract internal logics) and a human-readable username (for display on the
/// frontend).
#[grug::derive(Serde)]
pub struct UserIds {
    pub name: Option<Username>,
    pub index: UserIndex,
}

/// Information about a user. Used in query response.
#[grug::derive(Serde)]
pub struct User {
    /// Keys associated with this user, indexes by hashes.
    pub keys: BTreeMap<Hash256, Key>,
    /// Accounts associated with this user, indexes by addresses.
    pub accounts: BTreeMap<Addr, Account>,
}

/// Data the user must sign when onboarding. Currently, this consists of only
/// the chain ID.
#[grug::derive(Serde)]
pub struct RegisterUserData {
    pub chain_id: String,
}

impl SignData for RegisterUserData {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> StdResult<Vec<u8>> {
        self.to_json_value()?.to_json_vec()
    }
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
    /// Create a new user, following an initial deposit. Creates a spot account too.
    ///
    /// This is the second of the two-step user onboarding process.
    RegisterUser {
        key: Key,
        key_hash: Hash256,
        seed: u32,
        /// A signature over the `RegisterUserData`.
        signature: Signature,
    },
    /// Register a new account for an existing user.
    RegisterAccount { params: AccountParams },
    /// Associate a new or disassociate an existing key with a username.
    UpdateKey { key_hash: Hash256, key: Op<Key> },
    /// Update an account's parameters.
    UpdateAccount(AccountParamUpdates),
    /// Update the username.
    ///
    /// For now, we only support setting the username once when it's unset.
    /// We don't support changing the username when it's already set.
    UpdateUsername(Username),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the minimum deposit required to onboard a user.
    #[returns(Coins)]
    MinimumDeposit {},
    /// Query the next user index.
    #[returns(UserIndex)]
    NextUserIndex {},
    /// Query the next account index.
    #[returns(AccountIndex)]
    NextAccountIndex {},
    /// Query the code hash associated with the an account type.
    #[returns(Hash256)]
    CodeHash(AccountType),
    /// Enumerate all code hashes associated with account types.
    #[returns(BTreeMap<AccountType, Hash256>)]
    CodeHashes {
        start_after: Option<AccountType>,
        limit: Option<u32>,
    },
    /// Query a key by its hash and the user it is associated with.
    #[returns(Key)]
    Key { user: UserQuery, key_hash: Hash256 },
    /// Enumerate all keys.
    #[returns(Vec<QueryKeyResponseItem>)]
    Keys {
        start_after: Option<QueryKeyPaginateParam>,
        limit: Option<u32>,
    },
    /// Find all keys associated with a user.
    #[returns(BTreeMap<Hash256, Key>)]
    KeysByUser(UserQuery),
    /// Query parameters of an account by address.
    #[returns(Account)]
    Account(Addr),
    /// Enumerate all accounts and addresses.
    #[returns(BTreeMap<Addr, Account>)]
    Accounts {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    /// Find all accounts associated with a user.
    #[returns(BTreeMap<Addr, Account>)]
    AccountsByUser(UserQuery),
    /// Query a single user by its idenfier (either the index or the username).
    #[returns(User)]
    User(UserQuery),
    /// Query a user's username by index.
    ///
    /// `None` if the user index doesn't exist, or if the user index exists but
    /// its username is unset.
    #[returns(Option<Username>)]
    UserNameByIndex(UserIndex),
    /// Query a user's index by username.
    ///
    /// `None` if no user index is associated with this username.
    #[returns(Option<UserIndex>)]
    UserIndexByName(Username),
    /// Query user identifiers (index or username) associated with a given key hash.
    /// Useful if user forgot their username but still have access to the key.
    #[returns(Vec<UserIds>)]
    ForgotUsername {
        key_hash: Hash256,
        start_after: Option<UserQuery>,
        limit: Option<u32>,
    },
}

#[grug::derive(Serde)]
pub struct QueryKeyPaginateParam {
    pub user: UserQuery,
    pub key_hash: Hash256,
}

#[grug::derive(Serde)]
pub struct QueryKeyResponseItem {
    pub user: UserIds,
    pub key_hash: Hash256,
    pub key: Key,
}
