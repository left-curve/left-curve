use {
    crate::eth_utils,
    dango_types::{account_factory::Username, gateway::Domain},
    grug::{HexByteArray, Timestamp},
    hex_literal::hex,
    hyperlane_types::{Addr32, addr32},
    std::sync::LazyLock,
};

pub const MOCK_CHAIN_ID: &str = "mock-1";

pub const MOCK_GENESIS_TIMESTAMP: Timestamp = Timestamp::from_days(365);

macro_rules! mock_user {
    ($username:ident, $vk:literal, $sk:literal) => {
        pub mod $username {
            use super::*;

            pub const USERNAME: LazyLock<Username> = LazyLock::new(|| Username::new_unchecked(stringify!($username)));
            pub const PUBLIC_KEY: [u8; 33] = hex!($vk);
            pub const PRIVATE_KEY: [u8; 32] = hex!($sk);
        }
    };
    ($({ $username:ident, $vk:literal, $sk:literal }),* $(,)?) => {
        $(
            mock_user!($username, $vk, $sk);
        )*
    };
}

// Mock up accounts for testing purposes. See docs for the seed phrases used to
// generate these keys.
mock_user! {
    {
        owner,
        "0278f7b7d93da9b5a62e28434184d1c337c2c28d4ced291793215ab6ee89d7fff8",
        "8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177"
    },
    {
        user1,
        "03bcf89d5d4f18048f0662d359d17a2dbbb08a80b1705bc10c0b953f21fb9e1911",
        "a5122c0729c1fae8587e3cc07ae952cb77dfccc049efd5be1d2168cbe946ca18"
    },
    {
        user2,
        "02d309ba716f271b1083e24a0b9d438ef7ae0505f63451bc1183992511b3b1d52d",
        "cac7b4ced59cf0bfb14c373272dfb3d4447c7cd5aea732ea6ff69e19f85d34c4"
    },
    {
        user3,
        "024bd61d80a2a163e6deafc3676c734d29f1379cb2c416a32b57ceed24b922eba0",
        "cf6bb15648a3a24976e2eeffaae6201bc3e945335286d273bb491873ac7c3141"
    },
    {
        user4,
        "024a23e7a6f85e942a4dbedb871c366a1fdad6d0b84e670125991996134c270df2",
        "126b714bfe7ace5aac396aa63ff5c92c89a2d894debe699576006202c63a9cf6"
    },
    {
        user5,
        "03da86b1cd6fd20350a0b525118eef939477c0fe3f5052197cd6314ed72f9970ad",
        "fe55076e4b2c9ffea813951406e8142fefc85183ebda6222500572b0a92032a7"
    },
    {
        user6,
        "03428b179a075ff2142453c805a71a63b232400cc33c8e8437211e13e2bd1dec4c",
        "4d3658519dd8a8227764f64c6724b840ffe29f1ca456f5dfdd67f834e10aae34"
    },
    {
        user7,
        "028d4d7265d5838190842ada2573ef9edfc978dec97ca59ce48cf1dd19352a4407",
        "82de24ba8e1bc4511ae10ce3fbe84b4bb8d7d8abc9ba221d7d3cf7cd0a85131f"
    },
    {
        user8,
        "02a888b140a836cd71a5ef9bc7677a387a2a4272343cf40722ab9e85d5f8aa21bd",
        "ca956fcf6b0f32975f067e2deaf3bc1c8632be02ed628985105fd1afc94531b9"
    },
    {
        user9,
        "0230f93baa8e1dbe40a928144ec2144eed902c94b835420a6af4aafd2e88cb7b52",
        "c0d853951557d3bdec5add2ca8e03983fea2f50c6db0a45977990fb7b0c569b3"
    }
}

// TODO: update once we get a real domain.
pub const MOCK_HYPERLANE_DANGO_DOMAIN: Domain = 88888888;

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
