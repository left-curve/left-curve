//! Gateway is Dango's bridge aggregator contract. Dango will be equipped with
//! the following bridge protocols:
//!
//! - Hyperlane;
//! - Dango's own bitcoin bridge (name TBD).
//!
//! There are logics common to both protocols. Instead of duplicating them in
//! each protocol, they are extracted into the Gateway contract. These logics
//! include:
//!
//! - minting and burning of tokens;
//! - alloying;
//! - withdraw fee;
//! - withdraw rate limit.

pub mod bridge;
mod msgs;
mod remote;

pub use {
    hyperlane_types::{Addr32, mailbox::Domain},
    msgs::*,
    remote::*,
};

use {
    grug::{Bounded, Part, Udec128, ZeroInclusiveOneExclusive},
    std::sync::LazyLock,
};

pub type RateLimit = Bounded<Udec128, ZeroInclusiveOneExclusive>;

pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("bridge"));
