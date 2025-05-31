use {
    crate::{
        Addr, Binary, Coins, Config, Hash256, HashExt, Json, JsonSerExt, LengthBounded, MaxLength,
        NonEmpty, StdError, StdResult, btree_map,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeMap,
};
#[cfg(feature = "async-graphql")]
use {
    async_graphql::{InputType, InputValueResult, registry::Registry},
    std::borrow::Cow,
};

/// An arbitrary binary data used for deriving address when instantiating a
/// contract.
///
/// Must be no more than 82 bytes.
pub type Salt = MaxLength<Binary, 82>;

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
    pub msgs: NonEmpty<Vec<Message>>,
    pub data: Json,
    pub credential: Json,
}

impl Tx {
    pub fn tx_hash(&self) -> StdResult<Hash256> {
        Ok(self.to_json_vec()?.hash256())
    }
}

/// A transaction but without a gas limit or credential.
///
/// This is for using in gas simulation.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct UnsignedTx {
    pub sender: Addr,
    pub msgs: NonEmpty<Vec<Message>>,
    pub data: Json,
}

#[cfg(feature = "async-graphql")]
impl InputType for UnsignedTx {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        "UnsignedTx".into()
    }

    fn create_type_info(_registry: &mut Registry) -> String {
        "UnsignedTx".to_string()
    }

    fn parse(value: Option<async_graphql::Value>) -> InputValueResult<Self> {
        let value = value.ok_or_else(|| {
            async_graphql::InputValueError::expected_type(async_graphql::Value::Null)
        })?;

        let json_str =
            serde_json::to_string(&value).map_err(async_graphql::InputValueError::custom)?;

        serde_json::from_str(&json_str).map_err(async_graphql::InputValueError::custom)
    }

    fn to_value(&self) -> async_graphql::Value {
        let json_str = serde_json::to_string(self).unwrap();
        serde_json::from_str(&json_str).unwrap()
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
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
    /// Instantiate a new contract.
    Instantiate(MsgInstantiate),
    /// Execute a contract.
    Execute(MsgExecute),
    /// Update the code hash associated with a contract.
    Migrate(MsgMigrate),
}

impl Message {
    pub fn configure<T>(new_cfg: Option<Config>, new_app_cfg: Option<T>) -> StdResult<Self>
    where
        T: Serialize,
    {
        Ok(MsgConfigure {
            new_cfg,
            new_app_cfg: new_app_cfg.map(|t| t.to_json_value()).transpose()?,
        }
        .into())
    }

    pub fn transfer<C>(to: Addr, coins: C) -> StdResult<Self>
    where
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        Ok(Message::Transfer(btree_map! { to => coins.try_into()? }))
    }

    pub fn batch_transfer<I, C>(transfers: I) -> StdResult<Self>
    where
        I: IntoIterator<Item = (Addr, C)>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        transfers
            .into_iter()
            .map(|(to, coins)| {
                let coins = coins.try_into()?;
                Ok((to, coins))
            })
            .collect::<StdResult<_>>()
            .map(Message::Transfer)
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
    pub new_cfg: Option<Config>,
    pub new_app_cfg: Option<Json>,
}

// recipient => coins
pub type MsgTransfer = BTreeMap<Addr, Coins>;

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
