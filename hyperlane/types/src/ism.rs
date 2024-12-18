use grug::HexBinary;

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(())]
    Verify {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}
