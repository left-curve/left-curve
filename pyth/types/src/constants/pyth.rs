use {crate::PythId, hex_literal::hex};

pub const PYTH_URL: &str = "https://hermes.pyth.network";

pub const ATOM_USD_ID: PythId = PythId::from_inner(hex!(
    "b00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819"
));

pub const BCH_USD_ID: PythId = PythId::from_inner(hex!(
    "3dd2b63686a450ec7290df3a1e0b583c0481f651351edfa7636f39aed55cf8a3"
));

pub const BNB_USD_ID: PythId = PythId::from_inner(hex!(
    "2f95862b045670cd22bee3114c39763a4a08beeb663b145d283c31d7d1101c4f"
));

pub const BTC_USD_ID: PythId = PythId::from_inner(hex!(
    "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43"
));

pub const DOGE_USD_ID: PythId = PythId::from_inner(hex!(
    "dcef50dd0a4cd2dcc17e45df1676dcb336a11a61c69df7a0299b0150c672d25c"
));

pub const ETH_USD_ID: PythId = PythId::from_inner(hex!(
    "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace"
));

pub const LTC_USD_ID: PythId = PythId::from_inner(hex!(
    "6e3f3fa8253588df9326580180233eb791e03b443a3ba7a1d892e73874e19a54"
));

pub const SHIB_USD_ID: PythId = PythId::from_inner(hex!(
    "f0d57deca57b3da2fe63a493f4c25925fdfd8edf834b20f93e1f84dbd1504d4a"
));

pub const SOL_USD_ID: PythId = PythId::from_inner(hex!(
    "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"
));

pub const SUI_USD_ID: PythId = PythId::from_inner(hex!(
    "23d7315113f5b1d3ba7a83604c44b94d79f4fd69af77f804fc7f920a6dc65744"
));

pub const USDC_USD_ID: PythId = PythId::from_inner(hex!(
    "eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a"
));

pub const WBTC_USD_ID: PythId = PythId::from_inner(hex!(
    "c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33"
));

pub const XRP_USD_ID: PythId = PythId::from_inner(hex!(
    "ec5d399846a9209f3fe5881d70aae9268c94339ff9817e8d18ff19fa05eea1c8"
));
