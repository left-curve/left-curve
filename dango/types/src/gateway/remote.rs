use {
    grug::{Inner, PrimaryKey, RawKey, StdError, StdResult},
    hyperlane_types::{Addr32, mailbox::Domain},
};

#[grug::derive(Serde, Borsh)]
#[derive(Copy, PartialOrd, Ord)]
pub enum Remote {
    /// Indicates the token was received through Hyperlane's Warp protocol.
    Warp {
        origin_domain: Domain,
        sender: Addr32,
    },
    /// Indicates the token was received through Dango's proprietary bitcoin bridge.
    Bitcoin,
}

impl PrimaryKey for Remote {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<grug::RawKey> {
        let bytes = match self {
            Remote::Warp {
                origin_domain,
                sender,
            } => {
                // tag:           1 byte
                // origin domain: 4 bytes
                // sender:        4 bytes
                // totor:         9 bytes
                let mut bytes = Vec::with_capacity(9);
                bytes.push(0);
                bytes.extend(origin_domain.to_be_bytes());
                bytes.extend(sender.into_inner());
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
                if bytes.len() != 8 {
                    return Err(StdError::deserialize::<Self::Output, _>(
                        "key",
                        format!(
                            "incorrect byte length for warp remote! expecting: 8, got: {}",
                            bytes.len()
                        ),
                    ));
                }

                let origin_domain_raw = bytes[0..4].try_into().unwrap();
                let origin_domain = Domain::from_be_bytes(origin_domain_raw);

                let sender_raw = bytes[4..8].try_into().unwrap();
                let sender = Addr32::from_inner(sender_raw);

                Ok(Remote::Warp {
                    origin_domain,
                    sender,
                })
            },
            1 => {
                if !bytes.is_empty() {
                    return Err(StdError::deserialize::<Self::Output, _>(
                        "key",
                        format!(
                            "incorrect byte length for bitcoin remote! expecting: 0, got: {}",
                            bytes.len()
                        ),
                    ));
                }

                Ok(Remote::Bitcoin)
            },
            _ => Err(StdError::deserialize::<Self::Output, _>(
                "key",
                format!("unknown remote tag: {tag}"),
            )),
        }
    }
}
