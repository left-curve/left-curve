use {dango_order_book::Dimensionless, pyth_types::PythLazerSubscriptionDetails};

/// The oracle's price source record. Identical in shape to a Pyth Lazer
/// subscription, so we alias the upstream type directly.
pub type PriceSource = PythLazerSubscriptionDetails;

/// A price source together with the weight it carries when several sources are
/// combined into a single price for a denom.
///
/// Most assets are priced from exactly one source, which carries the full
/// weight. Commodities priced off futures markets (e.g. Brent, WTI, natural
/// gas) instead use two sources -- the front-month and next-month contracts --
/// whose weights shift from the former to the latter as the front contract
/// approaches maturity.
#[grug_types::derive(Serde)]
pub struct PriceSourceWithWeight {
    pub price_source: PriceSource,
    pub weight: Dimensionless,
}

impl PriceSourceWithWeight {
    /// Construct a single price source that carries the full weight. Used when
    /// a denom is priced from exactly one feed; the weight is irrelevant for a
    /// one-element weighted mean, so we pick the canonical value of one.
    pub fn single(price_source: PriceSource) -> Self {
        Self {
            price_source,
            weight: Dimensionless::new_int(1),
        }
    }
}
