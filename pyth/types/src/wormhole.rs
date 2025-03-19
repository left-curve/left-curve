use {
    crate::BytesAnalyzer,
    anyhow::{anyhow, ensure},
    data_encoding::BASE64,
    grug::{Api, BlockInfo, ByteArray, Hash160, Hash256, Inner, Map, NonZero, Storage, Timestamp},
    std::{collections::BTreeMap, ops::Deref},
};

pub type GuardianSetIndex = u32;

#[grug::derive(Serde, Borsh)]
pub struct GuardianSet {
    pub addresses: Vec<Hash160>,
    pub expiration_time: Option<NonZero<Timestamp>>,
}

impl GuardianSet {
    pub fn quorum(&self) -> usize {
        ((self.addresses.len() * 10 / 3) * 2) / 10 + 1
    }
}

#[grug::derive(Serde)]
pub struct GuardianSignature {
    pub id_recover: u8,
    pub signature: ByteArray<{ WormholeVaa::SIGNATURE_LEN - 1 }>,
}

impl GuardianSignature {
    pub fn new(raw_bytes: [u8; WormholeVaa::SIGNATURE_LEN]) -> anyhow::Result<Self> {
        let mut bytes = BytesAnalyzer::new(raw_bytes.into());

        let signature = bytes.next_chunk::<{ WormholeVaa::SIGNATURE_LEN - 1 }>()?;
        let id_recover = bytes.next_u8()?;

        Ok(GuardianSignature {
            id_recover,
            signature: ByteArray::from_inner(signature),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WormholeVaa {
    pub version: u8,
    pub guardian_set_index: GuardianSetIndex,
    pub hash: Hash256,
    pub timestamp: u32,
    pub nonce: u32,
    pub emitter_chain: u16,
    pub sequence: u64,
    pub consistency_level: u8,
    pub signatures: BTreeMap<u8, GuardianSignature>,
    pub emitter_address: [u8; 32],
    pub payload: Vec<u8>,
}

impl WormholeVaa {
    pub const HEADER_LEN: usize = 6;
    pub const SIGNATURE_LEN: usize = 65;

    /// Create a new Wormhole VAA from raw bytes.
    pub fn new<T>(api: &dyn Api, raw_bytes: T) -> anyhow::Result<Self>
    where
        T: Into<Vec<u8>>,
    {
        let mut bytes = BytesAnalyzer::new(raw_bytes.into());

        let version = bytes.next_u8()?;
        let guardian_set_index = bytes.next_u32()?;
        let len_signers = bytes.next_u8()?;

        let signatures = (0..len_signers)
            .map(|_| {
                let index = bytes.next_u8()?;
                let signature = bytes.next_chunk::<{ WormholeVaa::SIGNATURE_LEN }>()?;

                Ok((index, GuardianSignature::new(signature)?))
            })
            .collect::<anyhow::Result<BTreeMap<u8, GuardianSignature>>>()?;

        let hash = Hash256::from_inner(api.keccak256(&api.keccak256(bytes.deref())));

        let timestamp = bytes.next_u32()?;
        let nonce = bytes.next_u32()?;
        let emitter_chain = bytes.next_u16()?;
        let emitter_address = bytes.next_chunk::<32>()?;
        let sequence = bytes.next_u64()?;
        let consistency_level = bytes.next_u8()?;
        let payload = bytes.consume();

        Ok(WormholeVaa {
            version,
            guardian_set_index,
            signatures,
            hash,
            timestamp,
            nonce,
            emitter_chain,
            emitter_address,
            sequence,
            consistency_level,
            payload,
        })
    }

    /// Verify the Wormhole VAA.
    pub fn verify(
        self,
        storage: &dyn Storage,
        api: &dyn Api,
        block: BlockInfo,
        guardian_sets: Map<GuardianSetIndex, GuardianSet>,
    ) -> anyhow::Result<()> {
        ensure!(
            self.version == 1,
            "invalid VAA version: {} != 1",
            self.version
        );

        let guardian_set = guardian_sets.load(storage, self.guardian_set_index)?;

        if let Some(expiry) = guardian_set.expiration_time {
            ensure!(
                block.timestamp < expiry.into_inner(),
                "guardian set expired! {} >= {}",
                block.timestamp.into_seconds(),
                expiry.inner().into_seconds()
            );
        }

        ensure!(
            self.signatures.len() >= guardian_set.quorum(),
            "not enough signatures: {} < {}",
            self.signatures.len(),
            guardian_set.quorum()
        );

        for (index, sig) in self.signatures {
            let decompressed_point =
                api.secp256k1_pubkey_recover(&self.hash, &sig.signature, sig.id_recover, false)?;
            let prehash = &decompressed_point[1..];
            let hash = api.keccak256(prehash);
            let addr = &hash[12..];

            let info = guardian_set
                .addresses
                .get(index as usize)
                .ok_or_else(|| anyhow!("guardian not found in the guardian set"))?
                .into_inner();

            ensure!(
                addr == info,
                "recovered guardian address does not match: {} != {}",
                BASE64.encode(addr),
                BASE64.encode(&info),
            );
        }

        Ok(())
    }
}
