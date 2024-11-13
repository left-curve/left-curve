use grug::Addr;

use crate::lending::LendingAppConfig;

/// Application-specific configurations of the Dango chain.
#[grug::derive(Serde)]
pub struct AppConfig {
    pub addresses: AppAddresses,
    pub lending: LendingAppConfig,
}

/// Addresses of relevant Dango contracts.
#[grug::derive(Serde)]
pub struct AppAddresses {
    pub account_factory: Addr,
    pub ibc_transfer: Addr,
    pub lending: Addr,
    pub oracle: Addr,
}
