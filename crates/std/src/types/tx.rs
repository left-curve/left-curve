use {
    crate::{Addr, Binary, Coin, Hash},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Tx {
    pub sender:     Addr,
    pub msgs:       Vec<Message>,
    pub credential: Binary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    StoreCode {
        wasm_byte_code: Binary,
    },
    Instantiate {
        code_hash: Hash,
        msg:       Binary,
        salt:      Binary,
        funds:     Vec<Coin>,
        admin:     Option<Addr>,
    },
    Execute {
        contract: Addr,
        msg:      Binary,
        funds:    Vec<Coin>,
    },
}
