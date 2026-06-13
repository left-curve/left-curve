use grug_types::LengthBounded;

/// Metadata of a token.
#[grug_types::derive(Serde, Borsh)]
pub struct Metadata {
    // The length limits were arbitrarily chosen and can be adjusted.
    pub name: LengthBounded<String, 1, 32>,
    pub symbol: LengthBounded<String, 1, 16>,
    pub description: Option<LengthBounded<String, 1, 140>>,
    pub decimals: u32,
}
