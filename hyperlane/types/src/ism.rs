use grug::HexBinary;

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Here we deviate from the reference implementation, which returns a `bool`
    /// indicating whether verification is successful.
    /// The drawback of that approach is in case it's unsuccessful, the error
    /// message isn't caught. We would only get an uninformative "ism verify
    /// failed!" message:
    /// <https://github.com/many-things/cw-hyperlane/blob/d07e55e17c791a5f6557f114e3fb6cb433d9b800/contracts/core/mailbox/src/execute.rs#L205>.
    /// We instead abort with the full error message.
    #[returns(())]
    Verify {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}
