use {
    crate::{
        Addr, Binary, Coins, ConfigUpdates, Hash256, Json, JsonSerExt, LengthBounded, MaxLength,
        Op, StdError, StdResult,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeMap,
};

/// An arbitrary binary data used for deriving address when instantiating a
/// contract.
///
/// Must be no more than 70 bytes.
pub type Salt = MaxLength<Binary, 70>;

/// A human-readable string describing a contract. Can be optionally provided
/// during instantiation.
///
/// Must be non-empty and no more than 128 bytes.
pub type Label = LengthBounded<String, 1, 128>;

/// A transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Tx {
    pub sender: Addr,
    pub gas_limit: u64,
    pub msgs: Vec<Message>,
    pub data: Json,
    pub credential: Json,
}

/// A transaction but without a gas limit or credential.
///
/// This is for using in gas simulation.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct UnsignedTx {
    pub sender: Addr,
    pub msgs: Vec<Message>,
    pub data: Json,
}

/// A message.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    /// Update the chain- and app-level configurations.
    ///
    /// Only the `owner` is authorized to do this.
    Configure {
        updates: ConfigUpdates,
        app_updates: BTreeMap<String, Op<Json>>,
    },
    /// Send coins to the given recipient address.
    Transfer { to: Addr, coins: Coins },
    /// Upload a Wasm binary code and store it in the chain's state.
    Upload { code: Binary },
    /// Register a new account.
    Instantiate {
        code_hash: Hash256,
        msg: Json,
        salt: Salt,
        label: Option<Label>,
        admin: Option<Addr>,
        funds: Coins,
    },
    /// Execute a contract.
    Execute {
        contract: Addr,
        msg: Json,
        funds: Coins,
    },
    /// Update the `code_hash` associated with a contract.
    ///
    /// Only the contract's `admin` is authorized to do this. If the admin is
    /// set to `None`, no one can update the code hash.
    Migrate {
        contract: Addr,
        new_code_hash: Hash256,
        msg: Json,
    },
}

impl Message {
    pub fn configure(updates: ConfigUpdates, app_updates: BTreeMap<String, Op<Json>>) -> Self {
        Self::Configure {
            updates,
            app_updates,
        }
    }

    pub fn transfer<C>(to: Addr, coins: C) -> StdResult<Self>
    where
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        Ok(Self::Transfer {
            to,
            coins: coins.try_into()?,
        })
    }

    pub fn upload<B>(code: B) -> Self
    where
        B: Into<Binary>,
    {
        Self::Upload { code: code.into() }
    }

    pub fn instantiate<M, S, C, L>(
        code_hash: Hash256,
        msg: &M,
        salt: S,
        label: Option<L>,
        admin: Option<Addr>,
        funds: C,
    ) -> StdResult<Self>
    where
        M: Serialize,
        S: Into<Binary>,
        C: TryInto<Coins>,
        L: Into<String>,
        StdError: From<C::Error>,
    {
        Ok(Self::Instantiate {
            code_hash,
            msg: msg.to_json_value()?,
            salt: Salt::new(salt.into())?,
            label: label.map(|l| Label::new(l.into())).transpose()?,
            admin,
            funds: funds.try_into()?,
        })
    }

    pub fn execute<M, C>(contract: Addr, msg: &M, funds: C) -> StdResult<Self>
    where
        M: Serialize,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        Ok(Self::Execute {
            contract,
            msg: msg.to_json_value()?,
            funds: funds.try_into()?,
        })
    }

    pub fn migrate<M>(contract: Addr, new_code_hash: Hash256, msg: &M) -> StdResult<Self>
    where
        M: Serialize,
    {
        Ok(Self::Migrate {
            contract,
            new_code_hash,
            msg: msg.to_json_value()?,
        })
    }
}
