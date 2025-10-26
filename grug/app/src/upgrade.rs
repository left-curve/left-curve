use {
    crate::AppResult,
    grug_types::{BlockInfo, Storage, Upgrade},
};

pub struct UpgradeHandler<VM> {
    /// An optional message to be printed to tracing logs.
    pub metadata: Upgrade,
    /// An action to perform.
    /// The function takes the state storage and the VM instance as inputs, and returns empty.
    pub action: fn(Box<dyn Storage>, VM, BlockInfo) -> AppResult<()>,
}
