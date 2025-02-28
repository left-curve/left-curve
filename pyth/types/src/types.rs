use grug::{AddrEncoder, EncodedBytes};

pub type PythId = EncodedBytes<[u8; 32], AddrEncoder>;
