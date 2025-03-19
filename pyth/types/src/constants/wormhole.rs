use {
    crate::{GuardianSet, GuardianSetIndex},
    grug::{btree_map, Binary, Hash160, Inner},
    std::{collections::BTreeMap, str::FromStr, sync::LazyLock},
};

/// Index of the Wormhole guardian set as of November 4, 2024.
pub const GUARDIAN_SETS_INDEX: GuardianSetIndex = 4;

/// Addresses of the Wormhole guardian set as of November 4, 2024.
pub const GUARDIANS_ADDRESSES: [&str; 19] = [
    "WJO1p2w/c5ZFZIiFvczAbNcKPNM=",
    "/2y5Ulib3oYsJe9DkhMvudSkIVc=",
    "EU3oRgGTvfOi/PgfhqCXZfR2L9E=",
    "EHoAhrMtegl3kmogUTHYcx05y+s=",
    "jIKy/YL67ScR1Zrw8kmdFucm9rI=",
    "EbOXVsBCRBvm2GULabVOvnFeI0M=",
    "VM5bTTSPt0uVjolm4uw9vUlYp80=",
    "FefK8HxOPcjnxGn5LIzYj7gAWiA=",
    "dKO/kTlT1pUmDYi8GqJaTu42PvA=",
    "AArAB2cns1++otrCj+5cyw/qdo4=",
    "r0XO0Ta52eJJA0ZK6In1yKcj/BQ=",
    "+TEkt8c4hDy7iehkyGLDjN3Mz5U=",
    "0sw3pNwDao0jK0j2LN1HMUEvSJA=",
    "2nmPaJajMx9ktIwS0dV/2cvnCBE=",
    "caob4dNsr+OGeRD5nAnjR4mcGcM=",
    "gZK25zh8zXaCd8F9qxt6UCfAs88=",
    "F44hrS53rgZxFUnPux+cep2Alug=",
    "XhSH81UV0CqSdTUEqNdUcbn0nts=",
    "b768iY9APkdz6V/rFegMmpnINI0=",
];

/// Wormhole guardian sets indexed by guardian set indexes as of November 4, 2024.
pub static GUARDIAN_SETS: LazyLock<BTreeMap<GuardianSetIndex, GuardianSet>> = LazyLock::new(|| {
    btree_map! {
        GUARDIAN_SETS_INDEX => GuardianSet {
            addresses: GUARDIANS_ADDRESSES
                .into_iter()
                .map(|addr| {
                    let bytes = Binary::from_str(addr)
                        .unwrap()
                        .into_inner()
                        .try_into()
                        .unwrap();
                    Hash160::from_inner(bytes)
                })
                .collect(),
            expiration_time: None,
        },
    }
});
