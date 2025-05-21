use {
    grug::{Denom, Part},
    std::sync::LazyLock,
};

pub mod dango {
    use super::*;

    pub static DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["dango"]));

    pub const DECIMAL: u8 = 6;
}

macro_rules! define_denom {
    ($name:ident => $decimal:literal) => {
        pub mod $name {
            use super::*;

            pub static SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked(stringify!($name)));

            pub static DENOM: LazyLock<Denom> = LazyLock::new(|| {
                Denom::from_parts([crate::gateway::NAMESPACE.clone(), SUBDENOM.clone()]).unwrap()
            });

            pub const DECIMAL: u8 = $decimal;
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
    ltc  => 8,
    sol  => 9,
    usdc => 6,
    xrp  => 6
}
