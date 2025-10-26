use {
    crate::AppResult,
    grug_types::{BlockInfo, Storage, Upgrade},
};

pub enum UpgradeHandler<VM> {
    /// The old version of the app is to be set to this state.
    Halt { at_height: u64 },
    /// The new version of the app isto be set to this state.
    Upgrade {
        /// Metadata describing this upgrade.
        metadata: Upgrade,
        /// An action to perform.
        /// The function takes the state storage and the VM instance as inputs, and returns empty.
        action: fn(Box<dyn Storage>, VM, BlockInfo) -> AppResult<()>,
    },
}
