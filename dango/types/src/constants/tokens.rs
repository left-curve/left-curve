use {
    grug::Denom,
    std::{str::FromStr, sync::LazyLock},
};

pub static ATOM_DENOM: LazyLock<Denom> =
    LazyLock::new(|| Denom::from_str("hyp/atom/atom").unwrap());

pub static BCH_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/bch/bch").unwrap());

pub static BNB_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/bnb/bnb").unwrap());

pub static BTC_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/btc/btc").unwrap());

pub static DANGO_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("dango").unwrap());

pub static DOGE_DENOM: LazyLock<Denom> =
    LazyLock::new(|| Denom::from_str("hyp/doge/doge").unwrap());

pub static ETH_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/eth/eth").unwrap()); // TODO: update this to alloyed denom

pub static LTC_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/ltc/ltc").unwrap());

pub static SHIB_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/eth/shib").unwrap());

pub static SOL_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/sol/sol").unwrap());

pub static SUI_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/sui/sui").unwrap());

pub static USDC_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/eth/usdc").unwrap()); // TODO: update this to alloyed denom

pub static WBTC_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/eth/wbtc").unwrap());

pub static XRP_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("hyp/xrp/xrp").unwrap());
