use {
    super::{Remote, WarpRemote},
    crate::bitcoin,
    grug::Uint128,
};

/// Each bridge contract must implement this execute API.
#[grug::derive(Serde)]
pub enum ExecuteMsg {
    // NOTE: Bridge contract must ensure only the Gateway contract can call this.
    Bridge(BridgeMsg),
}

#[grug::derive(Serde)]
pub enum BridgeMsg {
    TransferRemote {
        req: TransferRemoteRequest,
        amount: Uint128,
    },
}

#[grug::derive(Serde)]
pub enum TransferRemoteRequest {
    Warp {
        warp_remote: WarpRemote,
        recipient: hyperlane_types::Addr32,
    },
    Bitcoin {
        recipient: bitcoin::BitcoinAddress,
    },
}

impl TransferRemoteRequest {
    pub fn to_remote(&self) -> Remote {
        match self {
            TransferRemoteRequest::Warp { warp_remote, .. } => Remote::Warp(*warp_remote),
            TransferRemoteRequest::Bitcoin { .. } => Remote::Bitcoin,
        }
    }
}
