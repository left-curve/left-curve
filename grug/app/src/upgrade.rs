use {
    crate::AppResult,
    grug_types::{BlockInfo, Storage},
};

pub struct UpgradeHandler<VM> {
    /// An optional message to be printed to tracing logs.
    pub description: Option<&'static str>,
    /// The block height at which the upgrade is to be performed.
    pub height: u64,
    /// An action to perform.
    /// The function takes the state storage and the VM instance as inputs, and returns empty.
    pub action: fn(Box<dyn Storage>, VM, BlockInfo) -> AppResult<()>,
}
