//! Generic order-book primitives, shared between perpetual, spot, and
//! prediction markets. This crate is the bottom layer of the Dango stack —
//! it depends only on `grug` and external crates, never on `dango-types`
//! or any contract crate.

mod decompose;
mod events;
mod liquidity_depth;
mod min_size;
mod price;
mod price_band;
mod slippage;
pub mod state;
mod target_price;
mod typed_number;
mod types;
mod volume;

pub use {
    decompose::*, events::*, liquidity_depth::*, min_size::*, price::*, price_band::*, slippage::*,
    target_price::*, typed_number::*, types::*, volume::*,
};
