use {
    crate::Addr32,
    anyhow::ensure,
    grug::{Addr, Hash256, HexBinary, Inner},
};

pub const MAILBOX_VERSION: u8 = 3;

pub type Domain = u32;

// ----------------------------------- types -----------------------------------

#[grug::derive(Serde)]
pub struct Message {
    pub version: u8,
    pub nonce: u32,
    pub origin_domain: Domain,
    pub sender: Addr32,
    pub destination_domain: Domain,
    pub recipient: Addr32,
    pub body: HexBinary,
}

impl Message {
    pub fn encode(&self) -> HexBinary {
        let mut buf = Vec::with_capacity(77 + self.body.len());
        buf.extend(self.version.to_be_bytes());
        buf.extend(self.nonce.to_be_bytes());
        buf.extend(self.origin_domain.to_be_bytes());
        buf.extend_from_slice(self.sender.inner());
        buf.extend(self.destination_domain.to_be_bytes());
        buf.extend_from_slice(self.recipient.inner());
        buf.extend_from_slice(&self.body);
        buf.into()
    }

    pub fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        ensure!(
            buf.len() >= 77,
            "mailbox message should be at least 77 bytes, got: {}",
            buf.len()
        );

        Ok(Self {
            version: buf[0],
            nonce: u32::from_be_bytes(buf[1..5].try_into().unwrap()),
            origin_domain: Domain::from_be_bytes(buf[5..9].try_into().unwrap()),
            sender: Addr32::from_inner(buf[9..41].try_into().unwrap()),
            destination_domain: Domain::from_be_bytes(buf[41..45].try_into().unwrap()),
            recipient: Addr32::from_inner(buf[45..77].try_into().unwrap()),
            body: buf[77..].to_vec().into(),
        })
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Config {
    // Domain registry: https://github.com/hyperlane-xyz/hyperlane-registry
    pub local_domain: Domain,
    // Note: this is typically set to the message ID multisig ISM.
    pub default_ism: Addr,
    // Note: this is typically set to the IGP (interchain gas paymaster) hook.
    // Users who don't want to pay IGP fee can compose a message that indicates
    // a dfferent hook other than the IGP.
    // For Dango, this will be set to a "flat rate fee" hook.
    pub default_hook: Addr,
    // Note: this is typically set to the Merkle tree hook, or an aggregate hook
    // that contains the Merkle tree hook.
    pub required_hook: Addr,
}

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Send a message.
    Dispatch {
        destination_domain: Domain,
        recipient: Addr32,
        body: HexBinary,
        metadata: Option<HexBinary>,
        hook: Option<Addr>,
    },
    /// Receive a message.
    Process {
        raw_message: HexBinary,
        raw_metadata: HexBinary,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the mailbox configuration.
    #[returns(Config)]
    Config {},
    /// Query the current nonce.
    #[returns(u32)]
    Nonce {},
    /// Query whether a message has been delivered.
    #[returns(bool)]
    Delivered { message_id: Hash256 },
}

// ---------------------------------- events -----------------------------------

#[grug::derive(Serde)]
#[grug::event("mailbox_dispatch")]
pub struct Dispatch(pub Message);

#[grug::derive(Serde)]
#[grug::event("mailbox_dispatch_id")]
pub struct DispatchId {
    pub message_id: Hash256,
}

#[grug::derive(Serde)]
#[grug::event("mailbox_process")]
pub struct Process {
    pub origin_domain: Domain,
    pub sender: Addr32,
    pub recipient: Addr32,
}

#[grug::derive(Serde)]
#[grug::event("mailbox_process_id")]
pub struct ProcessId {
    pub message_id: Hash256,
}
