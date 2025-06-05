use {
    grug::Addr,
    std::fmt::{self, Display},
};

#[grug::derive(Borsh)]
#[derive(Ord, PartialOrd)]
pub struct Address(Addr);

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl malachitebft_core_types::Address for Address {}
