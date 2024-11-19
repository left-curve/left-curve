/// Query messages for the spot account
#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the account's current sequence number.
    #[returns(u32)]
    Sequence {},
}
