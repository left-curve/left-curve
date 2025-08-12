use {
    grug::HexByteArray,
    hex_literal::hex,
    hyperlane_types::{Addr32, addr32, mailbox::Domain},
};

// TODO: update once we get a real domain.
pub const MOCK_HYPERLANE_LOCAL_DOMAIN: Domain = 88888888;

pub const MOCK_HYPERLANE_REMOTE_MERKLE_TREE: Addr32 =
    addr32!("0000000000000000000000000000000000000000000000000000000000000000");

/// Three Secp256k1 private keys that will act as the validator set for Hyperlane.
///
/// See commenst for the seed phrases used to generate these keys. Do not use
/// these keys in production!
pub const MOCK_HYPERLANE_VALIDATOR_SIGNING_KEYS: [eth_utils::SigningKey; 3] = [
    // swift slow expire warfare tired foster stable knife gasp wrong legal liquid tell obvious horror shadow margin various fiction chief cargo horse gravity goose
    hex!("b326ef3ac58801ce41e5318bbfa889ec21f829a3fc2c94f25203f9d0c4989c55"),
    // captain pluck round present sad galaxy ridge chat struggle under cinnamon diagram plate modify clever boost depart ordinary salmon liberty kite glide reduce you
    hex!("8a7791804f9135f1a16ef23b925cd8e19cdac8a5358f54770b703f2108252862"),
    // nest talk predict school erosion wheel journey news easy million language act frog fault ancient movie margin upper find wheel foster empty pilot tattoo
    hex!("91ebf77092e3bf73218ce3e8a8ca22f320e452ae6d0885032683bbf0dc496114"),
];

/// The Ethereum addresses corresponding to the above private keys.
pub const MOCK_HYPERLANE_VALIDATOR_ADDRESSES: [HexByteArray<20>; 3] = [
    HexByteArray::from_inner(hex!("4e5088dd05269194c9cdf30cd7a72a2ddd31b23c")),
    HexByteArray::from_inner(hex!("08afdf59ba845eb4d1c70cc83c5fb4ad6bc358b7")),
    HexByteArray::from_inner(hex!("92f7690869a6453ace4b5461f76367db902c2350")),
];
