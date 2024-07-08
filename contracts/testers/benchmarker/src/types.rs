use grug::{grug_derive, Binary};

#[grug_derive(serde, borsh)]
pub enum ExecuteTest {
    DoNothingVecu8 {
        msg: Vec<u8>,
    },
    DoNothingBinary {
        msg: Binary,
    },
    Math {
        iterations: u64,
    },

    Crypto {
        on_host: bool,
        crypto_api: CryptoApi,
    },
}

#[grug_derive(serde, borsh)]
pub enum CryptoApi {
    Sepc256k1verify {
        msg_hash: Vec<u8>,
        sig: Vec<u8>,
        pk: Vec<u8>,
    },
    Sha2_256 {
        msg: Vec<u8>,
    },
    Blake3 {
        msg: Vec<u8>,
    },
}
