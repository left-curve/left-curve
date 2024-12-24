use crate::{EncodedBytes, HashEncoder};

/// A shorthand for constructing a constant hash from a hex string.
///
/// This is equivalent to:
///
/// ```ignore
/// Hash::from_inner(hex_literal::hex!("..."))
/// ```
#[macro_export]
macro_rules! hash {
    ($hex:literal) => {
        $crate::Hash::from_inner($crate::__private::hex_literal::hex!($hex))
    };
}

/// A hash of a fixed length, in uppercase hex encoding.
pub type Hash<const N: usize> = EncodedBytes<[u8; N], HashEncoder>;

/// A 20-byte hash, in uppercase hex encoding.
pub type Hash160 = Hash<20>;

/// A 32-byte hash, in uppercase hex encoding.
pub type Hash256 = Hash<32>;

/// A 64-byte hash, in uppercase hex encoding.
pub type Hash512 = Hash<64>;

impl<const N: usize> Hash<N> {
    /// The length (number of bytes) of hashes.
    ///
    /// In Grug, we use SHA-256 hash everywhere, of which the length is 32 bytes.
    ///
    /// Do not confuse length in terms of bytes and in terms of ASCII characters.
    /// We use Hex encoding, which uses 2 ASCII characters per byte, so the
    /// ASCII length should be 64.
    pub const LENGTH: usize = N;
    /// A zeroed-out hash. Useful as mockups or placeholders.
    pub const ZERO: Self = Self::from_inner([0; N]);
}
