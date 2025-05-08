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
mod msg;
mod remote;

pub use {
    hyperlane_types::{Addr32, mailbox::Domain},
    msg::*,
    remote::*,
};

use {
    grug::{Bounded, Part, Udec128, ZeroInclusiveOneExclusive},
    std::sync::LazyLock,
};

pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("gateway"));

pub type RateLimit = Bounded<Udec128, ZeroInclusiveOneExclusive>;
