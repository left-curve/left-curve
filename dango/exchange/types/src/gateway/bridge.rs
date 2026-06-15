use {super::Remote, dango_hyperlane_types::Addr32, dango_math::Uint128};

/// Each bridge contract must implement this execute API.
#[dango_primitives::derive(Serde)]
pub enum ExecuteMsg {
    // NOTE: Bridge contract must ensure only the Gateway contract can call this.
    Bridge(BridgeMsg),
}

#[dango_primitives::derive(Serde)]
pub enum BridgeMsg {
    TransferRemote {
        remote: Remote,
        amount: Uint128,
        recipient: Addr32,
    },
}
