use {
    grug::{Binary, Inner, PrimaryKey, RawKey, StdError, StdResult},
    hyperlane_types::{Addr32, mailbox::Domain},
};

#[grug::derive(Serde, Borsh)]
#[derive(Copy, PartialOrd, Ord)]
pub enum Remote {
    /// Indicates the token was received through Hyperlane's Warp protocol.
    Warp { domain: Domain, contract: Addr32 },
    /// Indicates the token was received through Dango's proprietary bitcoin bridge.
    Bitcoin,
}

impl PrimaryKey for Remote {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<grug::RawKey<'_>> {
        let bytes = match self {
            Remote::Warp { domain, contract } => {
                // tag:           1 byte
                // origin domain: 4 bytes
                // sender:        32 bytes
                // total:         37 bytes
                let mut bytes = Vec::with_capacity(37);
                bytes.push(0);
                bytes.extend(domain.to_be_bytes());
                bytes.extend(contract.into_inner());
                bytes
            },
            Remote::Bitcoin => {
                // tag: 1 byte
                vec![1]
            },
        };

        vec![RawKey::Owned(bytes)]
    }

    fn from_slice(bytes: &[u8]) -> StdResult<Self::Output> {
        let (tag, bytes) = (bytes[0], &bytes[1..]);
        match tag {
            0 => {
                if bytes.len() != 36 {
                    return Err(StdError::deserialize::<Self::Output, _, Binary>(
                        "key",
                        format!(
                            "incorrect byte length for warp remote! expecting: 36, got: {}",
                            bytes.len()
                        ),
                        bytes.into(),
                    ));
                }

                let mut origin_domain_raw = [0_u8; 4];
                origin_domain_raw.copy_from_slice(&bytes[0..4]);
                let origin_domain = Domain::from_be_bytes(origin_domain_raw);

                let mut sender_raw = [0_u8; 32];
                sender_raw.copy_from_slice(&bytes[4..36]);
                let sender = Addr32::from_inner(sender_raw);

                Ok(Remote::Warp {
                    domain: origin_domain,
                    contract: sender,
                })
            },
            1 => {
                if !bytes.is_empty() {
                    return Err(StdError::deserialize::<Self::Output, _, Binary>(
                        "key",
                        format!(
                            "incorrect byte length for bitcoin remote! expecting: 0, got: {}",
                            bytes.len()
                        ),
                        bytes.into(),
                    ));
                }

                Ok(Remote::Bitcoin)
            },
            _ => Err(StdError::deserialize::<Self::Output, _, Binary>(
                "key",
                format!("unknown remote tag: {tag}"),
                bytes.into(),
            )),
        }
    }
}
