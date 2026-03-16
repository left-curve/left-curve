use {
    crate::{
        account_factory::{Account, AccountIndex, NewUserSalt, UserIndex, Username},
        auth::{Key, Signature},
    },
    grug::{Addr, Hash256, JsonSerExt, Op, SignData, StdError, StdResult},
    sha2::Sha256,
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub enum UserIndexOrName {
    Index(UserIndex),
    Name(Username),
}

/// Information about a user. Used in query response.
#[grug::derive(Serde, Borsh)]
pub struct User {
    /// The user's numerical index. Skipped in Borsh (it's the map key);
    /// populated by query handlers so JSON responses are self-contained.
    #[borsh(skip)]
    pub index: UserIndex,

    /// The user's username.
    pub name: Username,

    /// Accounts associated with this user, keyed by account index.
    /// A BTreeMap preserves creation-time ordering via key sort.
    pub accounts: BTreeMap<AccountIndex, Addr>,

    /// Keys associated with this user, indexes by hashes.
    pub keys: BTreeMap<Hash256, Key>,
}

impl User {
    /// Return the user's master account, i.e. the first account created for
    /// this user.
    ///
    /// Since `User::accounts` is a BTreeMap sorted by AccountIndex,
    /// the first entry is the earliest created account.
    pub fn master_account(&self) -> Addr {
        self.accounts
            .first_key_value()
            .map(|(_, addr)| *addr)
            .expect("the user to have at least one account")
    }
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
    /// Code hash to be associated with the Dango account contract.
    pub account_code_hash: Hash256,
    /// Users with associated key to set up during genesis.
    /// Each genesis user is to be associated with exactly one key.
    /// A single-signature account will be created for each genesis user.
    pub users: Vec<NewUserSalt>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Create a new user, following an initial deposit. Creates a single-signature
    /// account too.
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
    RegisterAccount {},
    /// Associate a new or disassociate an existing key with a username.
    UpdateKey { key_hash: Hash256, key: Op<Key> },
    /// Update the username.
    ///
    /// For now, we only support setting the username once when it's unset.
    /// We don't support changing the username when it's already set.
    UpdateUsername(Username),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the code hash associated with the Dango account contract.
    #[returns(Hash256)]
    CodeHash {},
    /// Query the next user index.
    #[returns(UserIndex)]
    NextUserIndex {},
    /// Query the next account index.
    #[returns(AccountIndex)]
    NextAccountIndex {},
    /// Query a single user by index or username.
    #[returns(User)]
    User(UserIndexOrName),
    /// Enumerate all users by indexes. Enumeration by usernames is not supported.
    #[returns(BTreeMap<UserIndex, User>)]
    Users {
        start_after: Option<UserIndex>,
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
    /// Query users associated with a given key hash.
    /// Useful if user forgot their username but still have access to the key.
    #[returns(Vec<User>)]
    ForgotUsername {
        key_hash: Hash256,
        start_after: Option<UserIndex>,
        limit: Option<u32>,
    },
}
