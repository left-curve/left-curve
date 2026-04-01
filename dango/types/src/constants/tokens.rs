use {
    grug::{Denom, Part},
    std::sync::LazyLock,
};

pub mod dango {
    use super::*;

    pub static DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["dango"]));

    pub const DECIMAL: u32 = 6;
}

macro_rules! define_denom {
    ($name:ident => $decimal:literal) => {
        pub mod $name {
            use super::*;

            pub static SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked(stringify!($name)));

            pub static DENOM: LazyLock<Denom> = LazyLock::new(|| {
                Denom::from_parts([crate::gateway::NAMESPACE.clone(), SUBDENOM.clone()]).unwrap()
            });

            pub const DECIMAL: u32 = $decimal;
        }
    };
    ($($name:ident => $decimal:literal),*) => {
        $(
            define_denom!($name => $decimal);
        )*
    };
}

define_denom! {
    atom => 6,
    bch  => 8,
    bnb  => 18,
    btc  => 8,
    doge => 8,
    eth  => 18,
    // HYPE has 8 decimals in HyperCore, 18 decimals in HyperEVM.
    // We do not support bridging spot HYPE anyways, so what we use here doesn't matter.
    // But ideally we support HyperCore, so putting 8 here for now.
    hype => 8,
    ltc  => 8,
    sol  => 9,
    usdc => 6,
    xrp  => 6
}

macro_rules! define_perp_denom {
    ($name:ident, $subdenom:literal => $decimal:expr) => {
        pub mod $name {
            use super::*;

            pub static SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked($subdenom));

            pub static DENOM: LazyLock<Denom> = LazyLock::new(|| {
                Denom::from_parts([crate::perps::NAMESPACE.clone(), SUBDENOM.clone()]).unwrap()
            });

            pub const DECIMAL: u32 = $decimal;
        }
    };
    ($($name:ident, $subdenom:literal => $decimal:expr),*) => {
        $(
            define_perp_denom!($name, $subdenom => $decimal);
        )*
    };
}

define_perp_denom! {
    perp_btc,  "btcusd"  => btc::DECIMAL,
    perp_eth,  "ethusd"  => eth::DECIMAL,
    perp_sol,  "solusd"  => sol::DECIMAL,
    perp_hype, "hypeusd" => hype::DECIMAL
}
