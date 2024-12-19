use {
    anyhow::ensure,
    grug::{Addr, EncodedBytes, HexEncoder, Inner},
};

/// Hyperlane addresses are left-padded to 32 bytes. See:
/// <https://docs.hyperlane.xyz/docs/reference/messaging/send#:~:text=Recipient%20addresses%20are%20left%2Dpadded>
#[grug::derive(Serde)]
#[derive(Copy)]
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
