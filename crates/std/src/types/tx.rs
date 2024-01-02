use {
    crate::{Addr, Binary, Coin, Hash},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Tx {
    pub sender:     Addr,
    pub msgs:       Vec<Message>,
    pub credential: Option<Binary>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum Query {
    Info {},
    WasmRaw {
        contract: Addr,
        key:      Binary,
    },
    WasmSmart {
        contract: Addr,
        msg:      Binary,
    },
}
