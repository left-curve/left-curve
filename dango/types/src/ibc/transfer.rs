use {
    grug::{Addr, Empty, Part},
    std::sync::LazyLock,
};

pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("ibc"));

pub type InstantiateMsg = Empty;

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Receive an IBC fungible token transfer.
    /// This mimics the behavior of ICS-20 receiving a packet.
    ///
    /// If the recipient is found, then simply send the coins to the recipient.
    /// Otherwise, the funds are held in the contract, waiting for the recipient
    /// to claim it during account creation.
    ReceiveTransfer { recipient: Addr },
}
