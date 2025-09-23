use {
    crate::oracle::{PrecisionedPrice, PriceSource},
    grug::{Binary, Denom, Timestamp},
    pyth_types::PriceUpdate,
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub price_sources: BTreeMap<Denom, PriceSource>,
    /// Pyth Lazer trusted signers: public keys and expiration timestamps.
    pub trusted_signers: BTreeMap<Binary, Timestamp>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Set the price sources for the given denoms.
    RegisterPriceSources(BTreeMap<Denom, PriceSource>),
    /// Set a trusted signer for Pyth Lazer.
    RegisterTrustedSigner {
        public_key: Binary,
        expires_at: Timestamp,
    },
    /// Remove a trusted signer for Pyth Lazer.
    RemoveTrustedSigner { public_key: Binary },
    /// Submit price data from Pyth Network.
    FeedPrices(PriceUpdate),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query Pyth Lazer trusted signers and their expiration times.
    #[returns(BTreeMap<Binary, Timestamp>)]
    TrustedSigners {
        start_after: Option<Binary>,
        limit: Option<u32>,
    },
    /// Query the price of the given denom.
    #[returns(PrecisionedPrice)]
    Price { denom: Denom },
    /// Enumerate the prices of all supported denoms.
    #[returns(BTreeMap<Denom, PrecisionedPrice>)]
    Prices {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// Query the price source of the given denom.
    #[returns(PriceSource)]
    PriceSource { denom: Denom },
    /// Enumerate the price sources of all supported denoms.
    #[returns(BTreeMap<Denom, PriceSource>)]
    PriceSources {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
}
