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
    Configure(MsgConfigure),
    /// Send coins to the given recipient address.
    Transfer(MsgTransfer),
    /// Upload a Wasm binary code and store it in the chain's state.
    Upload(MsgUpload),
    /// Register a new account.
    Instantiate(MsgInstantiate),
    /// Execute a contract.
    Execute(MsgExecute),
    /// Update the `code_hash` associated with a contract.
    Migrate(MsgMigrate),
}

impl Message {
    pub fn configure(updates: ConfigUpdates, app_updates: BTreeMap<String, Op<Json>>) -> Self {
        MsgConfigure {
            updates,
            app_updates,
        }
        .into()
    }

    pub fn transfer<C>(to: Addr, coins: C) -> StdResult<Self>
    where
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        Ok(MsgTransfer {
            to,
            coins: coins.try_into()?,
        }
        .into())
    }

    pub fn upload<B>(code: B) -> Self
    where
        B: Into<Binary>,
    {
        MsgUpload { code: code.into() }.into()
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
        Ok(MsgInstantiate {
            code_hash,
            msg: msg.to_json_value()?,
            salt: Salt::new(salt.into())?,
            label: label.map(|l| Label::new(l.into())).transpose()?,
            admin,
            funds: funds.try_into()?,
        }
        .into())
    }

    pub fn execute<M, C>(contract: Addr, msg: &M, funds: C) -> StdResult<Self>
    where
        M: Serialize,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        Ok(MsgExecute {
            contract,
            msg: msg.to_json_value()?,
            funds: funds.try_into()?,
        }
        .into())
    }

    pub fn migrate<M>(contract: Addr, new_code_hash: Hash256, msg: &M) -> StdResult<Self>
    where
        M: Serialize,
    {
        Ok(MsgMigrate {
            contract,
            new_code_hash,
            msg: msg.to_json_value()?,
        }
        .into())
    }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MsgConfigure {
    pub updates: ConfigUpdates,
    pub app_updates: BTreeMap<String, Op<Json>>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MsgTransfer {
    pub to: Addr,
    pub coins: Coins,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MsgUpload {
    pub code: Binary,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MsgInstantiate {
    pub code_hash: Hash256,
    pub msg: Json,
    pub salt: Salt,
    pub label: Option<Label>,
    pub admin: Option<Addr>,
    pub funds: Coins,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MsgExecute {
    pub contract: Addr,
    pub msg: Json,
    pub funds: Coins,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MsgMigrate {
    pub contract: Addr,
    pub new_code_hash: Hash256,
    pub msg: Json,
}

macro_rules! impl_into_message {
    ($variant:ident, $msg:ty) => {
        impl From<$msg> for Message {
            #[inline]
            fn from(msg: $msg) -> Self {
                Self::$variant(msg)
            }
        }
    };
    ($($variant:ident => $msg:ty),+ $(,)?) => {
        $(
            impl_into_message!($variant, $msg);
        )+
    };
}

impl_into_message! {
    Configure   => MsgConfigure,
    Transfer    => MsgTransfer,
    Upload      => MsgUpload,
    Instantiate => MsgInstantiate,
    Execute     => MsgExecute,
    Migrate     => MsgMigrate,
}
