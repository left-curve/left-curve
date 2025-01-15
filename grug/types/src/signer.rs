use crate::{Addr, Message, NonEmpty, StdResult, Tx, UnsignedTx};

/// Represents an object that has an onchain address.
pub trait Addressable {
    fn address(&self) -> Addr;
}

impl Addressable for Addr {
    fn address(&self) -> Addr {
        *self
    }
}

/// Represents an object that can sign transactions in a synchronous manner.
pub trait Signer: Addressable {
    /// Generate an unsigned transaction with the approapriate metadata.
    fn unsigned_transaction(
        &self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
    ) -> StdResult<UnsignedTx>;

    /// Sign a transaction.
    ///
    /// ## Notes:
    ///
    /// This function takes a mutable reference to self, because signing may be
    /// a stateful process, e.g. the signer may keep track of a nonce, and this
    /// state may need to be updated.
    fn sign_transaction(
        &mut self,
        msgs: NonEmpty<Vec<Message>>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx>;
}
