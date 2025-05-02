use grug::{Addr, Denom, Uint128};

use super::{DestinationAddr, DestinationChain};

#[grug::derive(Serde)]
#[grug::event("transfer_remote")]
pub struct TransferRemote {
    pub sender: Addr,
    pub destination_chain: DestinationChain,
    pub recipient: DestinationAddr,
    pub token: Denom,
    pub amount: Uint128,
}
