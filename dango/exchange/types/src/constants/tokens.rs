use {
    dango_primitives::{Denom, Part},
    std::sync::LazyLock,
};

pub mod dango {
    use super::*;

    pub static DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["dango"]));

    pub const DECIMAL: u32 = 6;
}

macro_rules! define_spot_denom {
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
    ($($name:ident => $decimal:literal),* $(,)?) => {
        $(
            define_spot_denom!($name => $decimal);
        )*
    };
}

define_spot_denom! {
    eth  => 18,
    usdc => 6,
}

macro_rules! define_perp_denom {
    ($name:ident => $subdenom:literal) => {
        pub mod $name {
            use super::*;

            pub static SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked($subdenom));

            pub static DENOM: LazyLock<Denom> = LazyLock::new(|| {
                Denom::from_parts([crate::perps::NAMESPACE.clone(), SUBDENOM.clone()]).unwrap()
            });
        }
    };
    ($($name:ident => $subdenom:literal),* $(,)?) => {
        $(
            define_perp_denom!($name => $subdenom);
        )*
    };
}

define_perp_denom! {
    perp_btc   => "btcusd",
    perp_eth   => "ethusd",
    perp_sol   => "solusd",
    perp_hype  => "hypeusd",
    perp_xau   => "xauusd",
    perp_xag   => "xagusd",
    perp_brent => "brentusd",
    perp_wti   => "wtiusd",
}
