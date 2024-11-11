use grug::Addr;

/// Application-specific configurations of the Dango chain.
#[grug::derive(Serde)]
pub struct AppConfig {
    pub addresses: AppAddresses,
}

/// Addresses of relevant Dango contracts.
#[grug::derive(Serde)]
pub struct AppAddresses {
    pub account_factory: Addr,
    pub ibc_transfer: Addr,
}
