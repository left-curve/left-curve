use {
    crate::{Addr, Message, StdResult, Tx},
    std::future::Future,
};

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
    // Notes:
    // 1. This function takes a mutable reference to self, because signing
    // may be a stateful process, e.g. the signer may keep track of a sequence
    // number, and this state may need to be updated.
    // 2. For now we require returning an `StdResult`. This may be too restricting.
    // Consider allowing custom error type in the future.
    fn sign_transaction(
        &mut self,
        msgs: Vec<Message>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx>;
}

/// Represents an object that can sign transactions in an asynchronous manner.
///
/// For example, it may need to query necessary data from an RPC node in order
/// to perform the signing, which can be async.
pub trait AsyncSigner: Addressable {
    fn sign_transaction(
        &mut self,
        msgs: Vec<Message>,
        chain_id: &str,
        gas_limit: u64,
    ) -> impl Future<Output = StdResult<Tx>>;
}

// A `Signer` is automatically also an `AsyncSigner`.
impl<T> AsyncSigner for T
where
    T: Signer,
{
    // Although we can't use the `async` keyword in trait definition, we can use
    // it here!
    async fn sign_transaction(
        &mut self,
        msgs: Vec<Message>,
        chain_id: &str,
        gas_limit: u64,
    ) -> StdResult<Tx> {
        // Simply call the synchronous `sign_transaction` method.
        self.sign_transaction(msgs, chain_id, gas_limit)
    }
}
