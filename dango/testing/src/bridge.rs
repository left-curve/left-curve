use hex_literal::hex;

macro_rules! mock_guardians {
    ($name:ident, $pk:literal, $sk:literal) => {
        pub mod $name {
            use super::*;

            pub const PUBLIC_KEY: [u8; 33] = hex!($pk);
            pub const PRIVATE_KEY: [u8; 32] = hex!($sk);
        }
    };
    ($({ $name:ident, $vk:literal, $sk:literal }),* $(,)?) => {
        $(
            mock_guardians!($name, $vk, $sk);
        )*
    };
}

mock_guardians! {
    {
        // sun recycle question fun cram crystal crunch body grant enforce october title viable alcohol gesture grunt express argue regular axis child upset snow enough
        guardian1,
        "029ba1aeddafb6ff65d403d50c0db0adbb8b5b3616c3bc75fb6fecd075327099f6",
        "46f182ee40948a74d05d6ca0585440dd43e90eeee3ef944b1ee34a1831753251"
    },
    {
        // help speed west above camp hockey ketchup public message liquid shield jealous sphere tell steak ripple pretty verify hedgehog initial solve foster mail anger
        guardian2,
        "03053780b7d8b3e7eb2771d7b9d43a946412e53fac90eadd46e214ccbea21eada6",
        "92cf588c3c0fafff9f5d1a68e750b4dcf8ba1947a03abb7c3c8b4fd47bb9a47e"

    },
    {
        // visa vendor essence parade silly render fence page donate moment plate empty icon lens monitor taxi edit much float myself dynamic blur venue strategy
        guardian3,
        "02f0bbe8928ab8d703e2e85093ee84ddfa9a0fdf48c443333098bd6188386bdb35",
        "3652fb8f593786a42a2d61e165db97dddd42fe6d9b61de671eee5abef3865dd4"
    },

}

pub const MOCK_BITCOIN_REGTEST_VAULT: &str =
    "bcrt1q4ga0r07vte2p638c8vh4fvpwjaln0qmxalffdkgeztl8l0act0xsvm7j9k";
