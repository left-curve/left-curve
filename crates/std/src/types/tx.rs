use {
    crate::{Addr, Binary, Coin},
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
    Instantiate {
        wasm_byte_code: Binary,
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
