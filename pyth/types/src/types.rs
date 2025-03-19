use grug::{AddrEncoder, Binary, EncodedBytes};

pub type PythId = EncodedBytes<[u8; 32], AddrEncoder>;

#[grug::derive(Serde)]
pub struct LatestVaaResponse {
    pub binary: LatestVaaBinaryResponse,
}

#[grug::derive(Serde)]
pub struct LatestVaaBinaryResponse {
    pub data: Vec<Binary>,
}
