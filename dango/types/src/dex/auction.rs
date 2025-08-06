#[grug::derive(Serde, Borsh)]
pub enum AuctionState {
    Ongoing,
    Paused(String),
}
