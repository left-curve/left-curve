use {
    anyhow::ensure,
    grug::{Addr, EncodedBytes, HexEncoder, Inner, PrimaryKey, RawKey, StdResult},
    std::fmt::{self, Display},
};

#[macro_export]
macro_rules! addr32 {
    ($hex_str:literal) => {
        $crate::Addr32::from_inner(::grug::__private::hex_literal::hex!($hex_str))
    };
}

/// Hyperlane addresses are left-padded to 32 bytes. See:
/// <https://docs.hyperlane.xyz/docs/reference/messaging/send#:~:text=Recipient%20addresses%20are%20left%2Dpadded>
#[grug::derive(Serde, Borsh)]
#[derive(Copy, PartialOrd, Ord)]
pub struct Addr32(EncodedBytes<[u8; 32], HexEncoder>);

impl Addr32 {
    pub const fn from_inner(inner: [u8; 32]) -> Self {
        Self(EncodedBytes::from_inner(inner))
    }
}

impl Inner for Addr32 {
    type U = [u8; 32];

    fn inner(&self) -> &Self::U {
        self.0.inner()
    }

    fn into_inner(self) -> Self::U {
        self.0.into_inner()
    }
}

impl From<Addr> for Addr32 {
    fn from(addr: Addr) -> Self {
        let mut addr32 = [0; 32];
        addr32[12..].copy_from_slice(&addr);
        Self(EncodedBytes::from_inner(addr32))
    }
}

impl TryFrom<Addr32> for Addr {
    type Error = anyhow::Error;

    fn try_from(addr32: Addr32) -> anyhow::Result<Self> {
        ensure!(
            addr32.0[..12].iter().all(|&b| b == 0),
            "invalid hyperlane address: left 12 bytes are not all zero"
        );

        let mut addr = [0; 20];
        addr.copy_from_slice(&addr32.0[12..]);

        Ok(Addr::from_inner(addr))
    }
}

impl PrimaryKey for Addr32 {
    type Output = Addr32;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<RawKey<'_>> {
        vec![RawKey::Borrowed(&self.0)]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        // TODO: more informative error message
        bytes.try_into().map(Self::from_inner).map_err(Into::into)
    }
}

impl Display for Addr32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
