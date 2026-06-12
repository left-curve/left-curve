use {
    crate::oracle::{Price, PriceConfig},
    grug_types::{Binary, Denom, Timestamp},
    pyth_types::PriceUpdate,
    std::collections::{BTreeMap, BTreeSet},
};

#[grug_types::derive(Serde)]
pub struct InstantiateMsg {
    pub price_sources: BTreeMap<Denom, PriceConfig>,

    /// Pyth Lazer trusted signers: public keys and expiration timestamps.
    pub trusted_signers: BTreeMap<Binary, Timestamp>,
}

#[grug_types::derive(Serde)]
pub enum ExecuteMsg {
    /// Set the price sources for the given denoms.
    RegisterPriceSources(BTreeMap<Denom, PriceConfig>),

    /// Remove the price sources for the given denoms.
    ///
    /// No check is performed on whether the denoms currently have a price
    /// source; removing a non-existent one is simply a no-op. Additionally,
    /// the owner is trusted to have ensured no other contract still relies
    /// on the price sources being removed.
    RemovePriceSources(BTreeSet<Denom>),

    /// Register a trusted signer for Pyth Lazer.
    RegisterTrustedSigner {
        public_key: Binary,
        expires_at: Timestamp,
    },

    /// Remove a trusted signer for Pyth Lazer.
    RemoveTrustedSigner { public_key: Binary },

    /// Submit price data from Pyth Network.
    FeedPrices(PriceUpdate),
}

#[grug_types::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query Pyth Lazer trusted signers and their expiration times.
    #[returns(BTreeMap<Binary, Timestamp>)]
    TrustedSigners {
        start_after: Option<Binary>,
        limit: Option<u32>,
    },

    /// Query the price of the given denom.
    #[returns(Price)]
    Price { denom: Denom },

    /// Enumerate the prices of all supported denoms.
    #[returns(BTreeMap<Denom, Price>)]
    Prices {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },

    /// Query the price config of the given denom.
    #[returns(PriceConfig)]
    PriceSource { denom: Denom },

    /// Enumerate the price configs of all supported denoms.
    #[returns(BTreeMap<Denom, PriceConfig>)]
    PriceSources {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
}
