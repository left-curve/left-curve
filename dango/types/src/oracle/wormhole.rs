use {
    super::BytesAnalyzer,
    anyhow::{anyhow, ensure},
    data_encoding::BASE64,
    grug::{
        Api, BlockInfo, ByteArray, Hash160, Hash256, HashExt, Inner, Map, NonZero, Storage,
        Timestamp,
    },
    serde::{
        de::{self, Visitor},
        Deserialize, Deserializer,
    },
    std::{collections::BTreeMap, fmt, ops::Deref, str::FromStr},
};

/// Addresses of the Wormhole guardian set as of November 4, 2024.
pub const GUARDIANS_ADDRESSES: [&str; 19] = [
    "WJO1p2w/c5ZFZIiFvczAbNcKPNM=",
    "/2y5Ulib3oYsJe9DkhMvudSkIVc=",
    "EU3oRgGTvfOi/PgfhqCXZfR2L9E=",
    "EHoAhrMtegl3kmogUTHYcx05y+s=",
    "jIKy/YL67ScR1Zrw8kmdFucm9rI=",
    "EbOXVsBCRBvm2GULabVOvnFeI0M=",
    "VM5bTTSPt0uVjolm4uw9vUlYp80=",
    "FefK8HxOPcjnxGn5LIzYj7gAWiA=",
    "dKO/kTlT1pUmDYi8GqJaTu42PvA=",
    "AArAB2cns1++otrCj+5cyw/qdo4=",
    "r0XO0Ta52eJJA0ZK6In1yKcj/BQ=",
    "+TEkt8c4hDy7iehkyGLDjN3Mz5U=",
    "0sw3pNwDao0jK0j2LN1HMUEvSJA=",
    "2nmPaJajMx9ktIwS0dV/2cvnCBE=",
    "caob4dNsr+OGeRD5nAnjR4mcGcM=",
    "gZK25zh8zXaCd8F9qxt6UCfAs88=",
    "F44hrS53rgZxFUnPux+cep2Alug=",
    "XhSH81UV0CqSdTUEqNdUcbn0nts=",
    "b768iY9APkdz6V/rFegMmpnINI0=",
];

/// Index of the Wormhole guardian set as of November 4, 2024.
pub const GUARDIAN_SETS_INDEX: u32 = 4;

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
        let id_recover = bytes.next_u8();

        Ok(GuardianSignature {
            id_recover,
            signature: ByteArray::from_inner(signature),
        })
    }
}

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct WormholeVaa {
    pub version: u8,
    pub guardian_set_index: u32,
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

    pub fn new<T>(raw_bytes: T) -> anyhow::Result<Self>
    where
        T: Into<Vec<u8>>,
    {
        let mut bytes = BytesAnalyzer::new(raw_bytes.into());

        let version = bytes.next_u8();
        let guardian_set_index = bytes.next_u32()?;
        let len_signers = bytes.next_u8();

        let signatures = (0..len_signers)
            .map(|_| {
                let index = bytes.next_u8();
                let signature = bytes.next_chunk::<{ WormholeVaa::SIGNATURE_LEN }>()?;

                Ok((index, GuardianSignature::new(signature)?))
            })
            .collect::<anyhow::Result<BTreeMap<u8, GuardianSignature>>>()?;

        // save some gas in wasm32
        #[cfg(not(target_arch = "wasm32"))]
        let hash = bytes.deref().keccak256().keccak256();
        #[cfg(target_arch = "wasm32")]
        let hash = Hash256::from_inner(
            grug::ExternalApi.keccak256(&grug::ExternalApi.keccak256(bytes.deref())),
        );

        let timestamp = bytes.next_u32()?;
        let nonce = bytes.next_u32()?;
        let emitter_chain = bytes.next_u16()?;
        let emitter_address = bytes.next_chunk::<32>()?;
        let sequence = bytes.next_u64()?;
        let consistency_level = bytes.next_u8();
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

    pub fn verify(
        self,
        storage: &dyn Storage,
        api: &dyn Api,
        block: BlockInfo,
        guardian_sets: Map<u32, GuardianSet>,
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

impl FromStr for WormholeVaa {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(BASE64.decode(s.as_bytes())?)
    }
}

impl<'de> Deserialize<'de> for WormholeVaa {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(WormholeVaaVisitor {})
    }
}

pub struct WormholeVaaVisitor;

impl<'de> Visitor<'de> for WormholeVaaVisitor {
    type Value = WormholeVaa;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("wormhole-vaa")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        WormholeVaa::from_str(v).map_err(E::custom)
    }
}
