use {
    crate::{mailbox::Domain, Addr32},
    anyhow::ensure,
    grug::{HexBinary, HexByteArray, Inner},
    std::collections::{BTreeMap, BTreeSet},
};

#[grug::derive(Serde, Borsh)]
pub struct ValidatorSet {
    pub threshold: u8,
    // A validator is identified by an Ethereum address. However we avoid using
    // the `Addr` type here (although we use the same address format as Ethereum)
    // to avoid confusion, as it's not a Grug/Dango address.
    pub validators: BTreeSet<HexByteArray<20>>,
}

#[grug::derive(Serde)]
pub struct Metadata {
    pub origin_merkle_tree: Addr32,
    pub merkle_root: HexByteArray<32>,
    pub merkle_index: u32,
    // The signatures aren't necessarily sorted, so use Vec instead of BTreeSet.
    pub signatures: Vec<HexByteArray<65>>,
}

impl Metadata {
    pub fn encode(&self) -> HexBinary {
        let mut buf = Vec::with_capacity(68 + self.signatures.len() * 65);
        buf.extend_from_slice(self.origin_merkle_tree.inner());
        buf.extend_from_slice(self.merkle_root.inner());
        buf.extend(self.merkle_index.to_be_bytes());
        for signature in &self.signatures {
            buf.extend_from_slice(signature.inner());
        }
        buf.into()
    }

    pub fn decode(buf: &[u8]) -> anyhow::Result<Self> {
        ensure!(
            buf.len() > 68,
            "multisig ISM metadata should be at least 68 bytes, got: {}",
            buf.len()
        );

        let signatures = buf[68..]
            .chunks_exact(65)
            .map(|chunk| HexByteArray::from_inner(chunk.try_into().unwrap()))
            .collect();

        Ok(Self {
            origin_merkle_tree: Addr32::from_inner(buf[0..32].try_into().unwrap()),
            merkle_root: HexByteArray::from_inner(buf[32..64].try_into().unwrap()),
            merkle_index: u32::from_be_bytes(buf[64..68].try_into().unwrap()),
            signatures,
        })
    }
}

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub validator_sets: BTreeMap<Domain, ValidatorSet>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Set validators for a domain.
    SetValidators {
        domain: Domain,
        threshold: u8,
        validators: BTreeSet<HexByteArray<20>>,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the validator set for a domain.
    #[returns(ValidatorSet)]
    ValidatorSet { domain: Domain },
    /// Enumerate validator sets of all domains.
    #[returns(BTreeMap<Domain, ValidatorSet>)]
    ValidatorSets {
        start_after: Option<Domain>,
        limit: Option<u32>,
    },
    /// Verify a message.
    /// Return nothing is succeeds; throw error if fails.
    #[returns(())]
    Verify {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}

// ---------------------------------- events -----------------------------------