use {
    crate::{
        Addr, Binary, Coins, ConfigUpdates, Hash256, Json, JsonSerExt, Label, Op, Salt, StdError,
        StdResult,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeMap,
};

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
        label: Label,
        funds: Coins,
        admin: Option<Addr>,
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
        label: L,
        funds: C,
        admin: Option<Addr>,
    ) -> StdResult<Self>
    where
        M: Serialize,
        S: Into<Salt>,
        C: TryInto<Coins>,
        L: Into<Label>,
        StdError: From<C::Error>,
    {
        Ok(Self::Instantiate {
            code_hash,
            msg: msg.to_json_value()?,
            salt: salt.into(),
            funds: funds.try_into()?,
            label: label.into(),
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use serde_json::Value;

    use crate::{BorshDeExt, BorshSerExt, Coins, Hash256, Label, Salt};

    use super::Message;

    #[test]
    fn borsh() {
        let msg = Message::Instantiate {
            code_hash: Hash256::from_array([0; 32]),
            msg: Value::default(),
            salt: Salt::from_str("tester").unwrap(),
            label: Label::from_str("tester").unwrap(),
            funds: Coins::default(),
            admin: None,
        };

        let ser = msg.to_borsh_vec().unwrap();
        let de: Message = ser.deserialize_borsh().unwrap();
    }
}
