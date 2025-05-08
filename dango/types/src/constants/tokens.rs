use {
    grug::{Denom, Part},
    std::sync::LazyLock,
};

pub mod atom {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("atom"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "atom"]));
}

pub mod bch {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("bch"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "bch"]));
}

pub mod bnb {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("bnb"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "bnb"]));
}

pub mod btc {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("btc"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "btc"]));
}

pub mod dango {
    use super::*;

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["dango"]));
}

pub mod doge {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("doge"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "doge"]));
}

pub mod eth {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("eth"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "eth"]));
}

pub mod ltc {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("ltc"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "ltc"]));
}

pub mod shib {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("shib"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "shib"]));
}

pub mod sol {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("sol"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "sol"]));
}

pub mod sui {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("sui"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "sui"]));
}

pub mod usdc {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("usdc"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "usdc"]));
}

pub mod wbtc {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("wbtc"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "wbtc"]));
}

pub mod xrp {
    use super::*;

    pub const SUBDENOM: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("xrp"));

    pub const DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["bridge", "xrp"]));
}
