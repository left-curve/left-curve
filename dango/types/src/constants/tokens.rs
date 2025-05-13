use {
    grug::{Denom, Part},
    std::sync::LazyLock,
};

pub mod dango {
    use super::*;

    pub static DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["dango"]));
}

macro_rules! define_denom {
    ($name:ident) => {
        pub mod $name {
            use super::*;

            pub static SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked(stringify!($name)));

            pub static DENOM: LazyLock<Denom> = LazyLock::new(|| {
                Denom::from_parts([crate::gateway::NAMESPACE.clone(), SUBDENOM.clone()]).unwrap()
            });
        }
    };
    ($($name:ident),*) => {
        $(
            define_denom!($name);
        )*
    };
}

define_denom!(atom, bch, bnb, btc, doge, eth, ltc, sol, usdc, xrp);
