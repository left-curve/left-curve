/// Three Secp256k1 private and public keys that will act as the guardian set for Bridge.
/// Do not use these keys in production!
pub const MOCK_BRIDGE_GUARDIANS_KEYS: [(&str, &str); 3] = [
    // sun recycle question fun cram crystal crunch body grant enforce october title viable alcohol gesture grunt express argue regular axis child upset snow enough
    // xprv tprv8ZgxMBicQKsPfPfzxKtpdnP3NPtwxtPqympBSRUA2bKnDdpMhdNxrJWqms3V6X2DYMTWCM8AAwqXTVkmFYQ1x7GyqR73LM67k473yXdGTke
    // xpub tpubDF2np9kJm9FyeyRC28VLoV7G1sBktuoMiPqVquPnbySPNDcFe2E3wmicwmPo63ExLajiumQstTZrq4K2Xaiyr7bSi7cAgrPkKF64wRLVH2Q
    // priv 46f182ee40948a74d05d6ca0585440dd43e90eeee3ef944b1ee34a1831753251
    // pub  029ba1aeddafb6ff65d403d50c0db0adbb8b5b3616c3bc75fb6fecd075327099f6
    (
        "46f182ee40948a74d05d6ca0585440dd43e90eeee3ef944b1ee34a1831753251",
        "029ba1aeddafb6ff65d403d50c0db0adbb8b5b3616c3bc75fb6fecd075327099f6",
    ),
    // help speed west above camp hockey ketchup public message liquid shield jealous sphere tell steak ripple pretty verify hedgehog initial solve foster mail anger
    // xprv tprv8ZgxMBicQKsPdF1scs3upJpazUGo8P9XPXiJrRSwnnzyBpwocgSqJJBxsNHLsDNcXjTApEVU8vvv2tYjLfFgYcabSYi9Gjo8xNY7CLrLycE
    // xpub tpubDEhdVbcHdejx56wtMuvMXyKPM9Y3yJbFZPxCsQpjSb6C6hgMVQcadMJCQ4TYacUDRdjpyK8Sfxuaa32DhFge8Na1eLnsEEPMqUTud86c5va
    // priv 92cf588c3c0fafff9f5d1a68e750b4dcf8ba1947a03abb7c3c8b4fd47bb9a47e
    // pub  03053780b7d8b3e7eb2771d7b9d43a946412e53fac90eadd46e214ccbea21eada6
    (
        "92cf588c3c0fafff9f5d1a68e750b4dcf8ba1947a03abb7c3c8b4fd47bb9a47e",
        "03053780b7d8b3e7eb2771d7b9d43a946412e53fac90eadd46e214ccbea21eada6",
    ),
    // visa vendor essence parade silly render fence page donate moment plate empty icon lens monitor taxi edit much float myself dynamic blur venue strategy
    // xprv tprv8ZgxMBicQKsPdzKCHDkFkwenM3xGB3PjzsfohuhAhnt5mWZFjVQgUixdEsGAANLj16GBTCUuy7XMDybbFfLF3D3DLGzv8YcYsjHp4XB7J7f
    // xpub tpubDEDwrNCUSXYiuSMsuJY3dNaNcis9cZfZsgyJSVYUkzzjXh8SU7LZhJD5E1JSrWgUTeCPhUu7q4Hf1r2XDFgW1EbniMUcPzHXXM5kRGouAQG
    // priv 3652fb8f593786a42a2d61e165db97dddd42fe6d9b61de671eee5abef3865dd4
    // pub  02f0bbe8928ab8d703e2e85093ee84ddfa9a0fdf48c443333098bd6188386bdb35
    (
        "3652fb8f593786a42a2d61e165db97dddd42fe6d9b61de671eee5abef3865dd4",
        "02f0bbe8928ab8d703e2e85093ee84ddfa9a0fdf48c443333098bd6188386bdb35",
    ),
];

pub const MOCK_BITCOIN_REGTEST_VAULT: &str =
    "bcrt1q4ga0r07vte2p638c8vh4fvpwjaln0qmxalffdkgeztl8l0act0xsvm7j9k";
