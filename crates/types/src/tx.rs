use {
    crate::{
        to_json_value, Addr, Binary, Coins, ConfigUpdates, Hash256, Json, Op, StdError, StdResult,
    },
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeMap,
    ts_rs::TS,
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, TS)]
#[ts(export)]
pub struct Tx {
    pub sender: Addr,
    pub gas_limit: u64,
    pub msgs: Vec<Message>,
    #[ts(type = "Record<string, any>")]
    pub data: Json,
    #[ts(type = "Record<string, any>")]
    pub credential: Json,
}

/// A transaction but without a gas limit or credential.
///
/// This is for using in gas simulation.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, TS)]
#[ts(export)]
pub struct UnsignedTx {
    pub sender: Addr,
    pub msgs: Vec<Message>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Message {
    /// Update the chain- and app-level configurations.
    ///
    /// Only the `owner` is authorized to do this.
    Configure {
        updates: ConfigUpdates,
        #[ts(type = "Record<string, any>")]
        app_updates: BTreeMap<String, Op<Json>>,
    },
    /// Send coins to the given recipient address.
    Transfer { to: Addr, coins: Coins },
    /// Upload a Wasm binary code and store it in the chain's state.
    Upload { code: Binary },
    /// Register a new account.
    Instantiate {
        #[ts(type = "string")]
        code_hash: Hash256,
        #[ts(type = "Record<string, any>")]
        msg: Json,
        salt: Binary,
        funds: Coins,
        admin: Option<Addr>,
    },
    /// Execute a contract.
    Execute {
        contract: Addr,
        #[ts(type = "Record<string, any>")]
        msg: Json,
        funds: Coins,
    },
    /// Update the `code_hash` associated with a contract.
    ///
    /// Only the contract's `admin` is authorized to do this. If the admin is
    /// set to `None`, no one can update the code hash.
    Migrate {
        contract: Addr,
        #[ts(type = "string")]
        new_code_hash: Hash256,
        #[ts(type = "Record<string, any>")]
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

    pub fn instantiate<M, S, C>(
        code_hash: Hash256,
        msg: &M,
        salt: S,
        funds: C,
        admin: Option<Addr>,
    ) -> StdResult<Self>
    where
        M: Serialize,
        S: Into<Binary>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        Ok(Self::Instantiate {
            code_hash,
            msg: to_json_value(msg)?,
            salt: salt.into(),
            funds: funds.try_into()?,
            admin,
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
            msg: to_json_value(msg)?,
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
            msg: to_json_value(msg)?,
        })
    }
}
