/// The oracle's price source record. Identical in shape to a Pyth Lazer
/// subscription, so we alias the upstream type directly.
pub type PriceSource = pyth_types::PythLazerSubscriptionDetails;
