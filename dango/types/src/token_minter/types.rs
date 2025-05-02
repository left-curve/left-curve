use {
    grug::{Addr, Bounded, PrimaryKey, RawKey, StdError, Udec128, ZeroInclusiveOneExclusive},
    hyperlane_types::Addr32,
};

pub type RateLimit = Bounded<Udec128, ZeroInclusiveOneExclusive>;

#[grug::derive(Serde)]
pub enum TransferMetadata {}

#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub enum DestinationChain {
    Hyperlane { domain: u32 },
    Bitcoin {},
}

#[grug::derive(Serde)]
#[derive(Copy)]
pub enum DestinationAddr {
    Hyperlane(Addr32),
    Bitcoin(Addr),
}

impl PrimaryKey for DestinationChain {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 2;

    fn raw_keys(&self) -> Vec<RawKey> {
        match self {
            DestinationChain::Hyperlane { domain } => {
                vec![RawKey::Fixed8([0]), RawKey::Fixed32(domain.to_be_bytes())]
            },
            DestinationChain::Bitcoin {} => {
                vec![RawKey::Fixed8([1]), RawKey::Fixed32([0; 4])]
            },
        }
    }

    fn from_slice(bytes: &[u8]) -> grug::StdResult<Self::Output> {
        match &bytes[..3] {
            // Hyperlane
            [0, 1, 0] => {
                let domain = u32::from_be_bytes(bytes[3..].try_into()?);
                Ok(DestinationChain::Hyperlane { domain })
            },
            // Bitcoin
            [0, 1, 1] => Ok(DestinationChain::Bitcoin {}),
            tag => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                format!("unknown tag: {tag:?}"),
            )),
        }
    }
}

impl PartialEq<DestinationChain> for DestinationAddr {
    fn eq(&self, other: &DestinationChain) -> bool {
        match (self, other) {
            (DestinationAddr::Hyperlane(_), DestinationChain::Hyperlane { .. }) => true,
            (DestinationAddr::Bitcoin(_), DestinationChain::Bitcoin {}) => true,
            _ => false,
        }
    }
}
