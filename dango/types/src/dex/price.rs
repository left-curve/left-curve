use grug::Udec128_24;

/// Through the DEX contract, we use 24 decimal places for prices.
///
/// ## Developer's note
///
/// The reason for this is to accomodate for assets with low prices and high
/// decimal places. Let's take the SHIB token for example. It has 18 decimal
/// places, and as I'm writing this, it's trading at 0.00001267 USDC (1.267e-5)
/// per token.
///
/// In our DEX, prices are not represented in human units (USDC per SHIB), but
/// in base units. For SHIB, the base unit is 1e-18 SHIB. For USDC, it's 1e-6 USDC.
/// So the price would be:
///
/// ```plain
/// 1.267e-5 * 1e-18 / 1e-6 = 1.267e-17
/// ```
///
/// If we use the default 18 decimal places, this becomes 0.000000000000000012,
/// with only 2 significant digits left. This is too little to accurately do any
/// math. As such, we need more decimal places. We decided 24 should be sufficient
/// under any reasonable circumstances.
///
/// Then the question is, do we use 128 bits or 256 bits? With 128 bits, the
/// maximum value we can represent is:
///
/// ```plain
/// (2^128 - 1) / 10^24 = 3.4e+14
/// ```
///
/// Suppose ETH is trading at 4000 USDC (or in other words, 4e-15 in base units),
/// this corresponds to 3.4e+14 / 4e-15 = 8.5e+28 wei or 8.5e+10 ETH, way beyond
/// the biggest order size we reasonable expect. So, 128-bit is sufficient.
pub type Price = Udec128_24;

/// The state of resting order book.
///
/// Resting order book is defined as the order book after the last block's auction,
/// before any of the order creation or cancelation of this block has been applied.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct RestingOrderBookState {
    /// The highest available bid price after the last block's auction.
    /// `None` if no bids were left after the auction.
    pub best_bid_price: Option<Price>,
    /// The lowest available ask price after the last block's auction.
    /// `None` if no asks were left after the auction.
    pub best_ask_price: Option<Price>,
    /// The middle point between the best bid and ask prices, if both are available.
    /// If only one side is available, the best price on that side.
    /// `None` if neither side is available.
    pub mid_price: Option<Price>,
}
