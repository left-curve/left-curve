use {
    grug::{Addr, PrimaryKey},
    std::fmt::{self, Display},
};

#[grug::derive(Borsh)]
#[derive(Copy, Ord, PartialOrd)]
pub struct Address(Addr);

impl Address {
    pub fn new(addr: Addr) -> Self {
        Self(addr)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl malachitebft_core_types::Address for Address {}

impl PrimaryKey for Address {
    type Output = Address;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<grug::RawKey> {
        self.0.raw_keys()
    }

    fn from_slice(bytes: &[u8]) -> grug::StdResult<Self::Output> {
        Ok(Self(Addr::from_slice(bytes)?))
    }
}
