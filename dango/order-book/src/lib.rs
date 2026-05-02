//! Generic order-book primitives, shared between perpetual, spot, and
//! prediction markets. This crate is the bottom layer of the Dango stack —
//! it depends only on `grug` and external crates, never on `dango-types`
//! or any contract crate.

mod decompose;
mod min_size;
mod price;
mod price_band;
mod slippage;
mod typed_number;

pub use {decompose::*, min_size::*, price::*, price_band::*, slippage::*, typed_number::*};
