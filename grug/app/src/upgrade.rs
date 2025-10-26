use {
    crate::AppResult,
    grug_types::{BlockInfo, Storage, Upgrade},
};

#[derive(Clone)]
pub enum UpgradeHandler<VM> {
    /// Indicates the chain is to be halted at the given height.
    /// The old version of the app is to be set to this state.
    Halt(u64),
    /// Indicates the chain is to perform an upgrade as soon as it recovers from
    /// the earlier planned halt.
    /// The new version of the app is to be set to this state.
    Upgrade {
        /// Metadata describing this upgrade.
        metadata: Upgrade,
        /// An action to perform.
        /// The function takes the state storage and the VM instance as inputs, and returns empty.
        action: fn(Box<dyn Storage>, VM, BlockInfo) -> AppResult<()>,
    },
}
