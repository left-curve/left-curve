use {
    super::BytesAnalyzer,
    anyhow::{bail, ensure},
    grug::{
        Api, Binary, BlockInfo, ByteArray, Hash160, Hash256, HashExt, Inner, Map, StdError,
        StdResult, Storage,
    },
    k256::{
        elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint},
        AffinePoint, EncodedPoint,
    },
    serde::{de::Visitor, Deserialize},
    std::{collections::BTreeMap, ops::Deref, str::FromStr},
};

#[grug::derive(Serde, Borsh)]
pub struct GuardianSetInfo {
    pub addresses: Vec<Hash160>,
    pub expiration_time: u32,
}

impl GuardianSetInfo {
    pub fn quorum(&self) -> usize {
        ((self.addresses.len() * 10 / 3) * 2) / 10 + 1
    }
}

#[derive(Debug)]
pub struct GuardianSignature {
    pub id_recover: u8,
    pub signature: ByteArray<{ VAA::SIGNATURE_LEN - 1 }>,
}

impl GuardianSignature {
    pub fn new<T>(raw_bytes: T) -> StdResult<Self>
    where
        T: Into<Vec<u8>>,
    {
        let mut bytes = BytesAnalyzer::new(raw_bytes.into());

        let signature = bytes.next_bytes::<{ VAA::SIGNATURE_LEN - 1 }>()?;
        let id_recover = bytes.next_u8();

        Ok(GuardianSignature {
            id_recover,
            signature: ByteArray::from_inner(signature),
        })
    }
}

#[derive(Debug)]
pub struct VAA {
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

impl VAA {
    pub const HEADER_LEN: usize = 6;
    pub const SIGNATURE_LEN: usize = 65;

    pub fn new<T>(raw_bytes: T) -> StdResult<Self>
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
                let signature = bytes.next_bytes::<{ VAA::SIGNATURE_LEN }>()?;
                Ok((index, GuardianSignature::new(signature)?))
            })
            .collect::<StdResult<BTreeMap<u8, GuardianSignature>>>()?;

        // We should use api functions but we are inside a trait, can't use it.
        // For now use the HashExt trait directly.
        // This need double hash
        let hash = bytes.deref().keccak256().keccak256();

        let timestamp = bytes.next_u32()?;
        let nonce = bytes.next_u32()?;
        let emitter_chain = bytes.next_u16()?;
        let emitter_address = bytes.next_bytes::<32>()?;
        let sequence = bytes.next_u64()?;
        let consistency_level = bytes.next_u8();

        Ok(VAA {
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
            payload: bytes.consume(),
        })
    }

    pub fn verify(
        self,
        storage: &dyn Storage,
        api: &dyn Api,
        block: BlockInfo,
        guardian_set: Map<u32, GuardianSetInfo>,
    ) -> anyhow::Result<()> {
        ensure!(self.version == 1, "Invalid VAA version");

        let guardian_set = guardian_set.load(storage, self.guardian_set_index)?;

        ensure!(
            guardian_set.expiration_time != 0
                && guardian_set.expiration_time as u128 > block.timestamp.into_inner().into_inner(),
            "Guardian set expired"
        );

        ensure!(
            guardian_set.quorum() <= self.signatures.len(),
            "Not enough signatures"
        );

        for (index, sign) in self.signatures {
            let pk =
                api.secp256k1_pubkey_recover(&self.hash, &sign.signature, sign.id_recover, true)?;

            let affine_point_option =
                AffinePoint::from_encoded_point(&EncodedPoint::from_bytes(pk)?);
            let affine_point = if affine_point_option.is_some().into() {
                affine_point_option.unwrap()
            } else {
                bail!("Encoded point not on the curve");
            };

            let decompressed_point = affine_point.to_encoded_point(false);
            let prehash = &decompressed_point.as_bytes()[1..];
            let addr = &prehash.keccak256()[12..];

            let info = guardian_set
                .addresses
                .get(index as usize)
                .ok_or_else(|| anyhow::anyhow!("Guardian not found in the guardian set"))?
                .into_inner();

            ensure!(addr == info, "Invalid signature");
        }

        Ok(())
    }
}

impl FromStr for VAA {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(Binary::from_str(s)?.into_inner())
    }
}

impl<'de> Deserialize<'de> for VAA {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(VAAVisitor {})
    }
}

pub struct VAAVisitor;

impl<'de> Visitor<'de> for VAAVisitor {
    type Value = VAA;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("vaa")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        VAA::from_str(v).map_err(E::custom)
    }
}

#[cfg(test)]
mod tests {

    use {
        super::*,
        crate::oracle::PythVaa,
        grug::{Duration, MockApi, MockStorage},
    };

    const VAA: &str = "UE5BVQEAAAADuAEAAAAEDQBkMyJzGWOwAlhd3NDvcYJvct5KACRi6oi9InIE/PYqXh1z92MOXFyFPGP5y9uOpubgMIvUh/pa5aXsM/z+aaCdAALKQlwSVB5YIQ/C0NuqXqam0fAAQYUJeBe+G7rjnv7UXhHRIqNiqCvTE1ygz3zUztg07pqoYahCI7SlqI23hHizAAPG7cQdoENAUMDgYC1znnRkG8NUDS/Yzlxb3Krl/fKDUjpgKM2ZEB5HD11bCTzIhPHTI8KQxIDbyKxF6o4cwf5QAAQxrIWXQX0Bx9/lDEDfFOOqRU6LwZhFMmiDwUedUxsIvR73V/yfZKNtObHA0O9McjdTo1JibRqnbNqw6H8hw4/JAAax4DOJ/M8yxbIk88rV0n8sttzelXPuMnnJCXV2CFpwlSqYu0cQ+gmWvfjK/zJSFKHhNF0N7wzOX9J/bghUeQ8nAQgJ7BPYtJo/qowTuQfDCa4ZHIhLjC9frRQh3/UWLrxosG5xWODfYWtpDLKwfmi2gjMV4PIMUdhwZLyMDfZIqR6MAQrB/IQ438iz+1cgU+i8ij7eB5+MeUxcV0ukQhJW/0nwVCm234OqZ+ES3fNPIpWHRo4nq5ZVCdX4ZE3MF+SjZIW2AAu4DFxPpw3tokuOP6z2jNk9AFzjC/WUqlZaIx+6Se5ZeGr4chhEh2IiwChhSUJnGsKtkXHSqTuLZpXf8QZ+ZiRFAAz9XiWxbiOvw6E4+I/0JRutYrALssiRNYBah4I1QzYSU1gIAeMEHz2jvMX9lGGZMfS/uJrv1VtW9UCJMxMCUqgOAA2Hkv95hjyj6toIigG6PyEpzzoJE3ZVqI92F2kWoGSE0l/7aV/sz6jhRl8udbq/Mqu+i9wpbUZqa/ZUCFFi0NLSAQ5s3Le7hPfK1QnMOU8eWkJqiy/XL+remqBwR92Omm8FFANUVzHwOKBsj0Zlrp9o7UW05BJUrUgVXbvJ61r2F+zoAREVSnZt5Tt3JOQs/JRFUway6AvKiQQJihLAOo6AkKiUCTR2G4kbFGiILq4hwgASZGshfdgKRCy+jbHlfDGpNF+vABIwoeTGgkil6kOH/Dg+hNKmqS8N41Y1tQn7i7RkfjMw7gMOQoZcNTKDCNGfgR0gu62ZIkDBIXmea25leCk6VnH2AGcgG4EAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFVzmdAUFVV1YAAAAAAApj+2QAACcQuyA5y12P+HQ9xkG4YvVJJeqDZf4BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZaLZ4aygAAAAIyAxQV////+AAAAABnIBuBAAAAAGcgG4AAAAZXwuHPYAAAAAJwWNtUCsIlij3mTR7FLM4Pu9qzDhJrUtUxIctFWnmj84Af485oCfcURBzjS8v9xlCaHMjofeED+Ml66aUMg3GKE8PDVhr5SAP4MJU436Fr6IFOxCWwq4hIuPuRgtLh6xy3t1dAZmA1SLzhr+OAOS1cKUapaSIeOdv/Mclu2fbSsnRU72f3eNeVU1v13bHKNJ70zxX/fMj109FD2kNQf4+VnjXn0jbxUKWfH5PZBT9oXoD9C59CFRYhLKAuMLSgi1sRBH0T1SmF59vcZjsn";

    const GUARDIAN_SETS: Map<u32, GuardianSetInfo> = Map::new("guardian_sets");

    const GUARDIANS_ADDRESSES: [&str; 19] = [
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

    const GUARDIAN_SETS_INDEX: u32 = 4;

    fn populate_guardian_set() -> MockStorage {
        let mut storage = MockStorage::new();

        let guardian_set = GuardianSetInfo {
            addresses: GUARDIANS_ADDRESSES
                .into_iter()
                .map(|val| {
                    let b = Binary::from_str(val).unwrap().into_inner();

                    Hash160::from_inner(b.try_into().unwrap())
                })
                .collect(),
            expiration_time: 100,
        };

        GUARDIAN_SETS
            .save(&mut storage, GUARDIAN_SETS_INDEX, &guardian_set)
            .unwrap();

        storage
    }

    #[test]
    fn byte_analizer() {
        let raw = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

        let mut analizer = BytesAnalyzer::new(raw);

        assert_eq!(analizer.next_u8(), 1);
        assert_eq!(analizer.next_u16().unwrap(), u16::from_be_bytes([2, 3]));
        assert_eq!(
            analizer.next_u32().unwrap(),
            u32::from_be_bytes([4, 5, 6, 7])
        );

        // deref
        assert_eq!(analizer.deref(), &[8, 9, 10, 11, 12, 13, 14, 15]);

        assert_eq!(analizer.next_bytes::<4>().unwrap(), [8, 9, 10, 11]);
        assert_eq!(analizer.consume(), vec![12, 13, 14, 15]);
    }

    #[test]
    fn des_vaa() {
        let str = r#""UE5BVQEAAAADuAEAAAAEDQBkMyJzGWOwAlhd3NDvcYJvct5KACRi6oi9InIE/PYqXh1z92MOXFyFPGP5y9uOpubgMIvUh/pa5aXsM/z+aaCdAALKQlwSVB5YIQ/C0NuqXqam0fAAQYUJeBe+G7rjnv7UXhHRIqNiqCvTE1ygz3zUztg07pqoYahCI7SlqI23hHizAAPG7cQdoENAUMDgYC1znnRkG8NUDS/Yzlxb3Krl/fKDUjpgKM2ZEB5HD11bCTzIhPHTI8KQxIDbyKxF6o4cwf5QAAQxrIWXQX0Bx9/lDEDfFOOqRU6LwZhFMmiDwUedUxsIvR73V/yfZKNtObHA0O9McjdTo1JibRqnbNqw6H8hw4/JAAax4DOJ/M8yxbIk88rV0n8sttzelXPuMnnJCXV2CFpwlSqYu0cQ+gmWvfjK/zJSFKHhNF0N7wzOX9J/bghUeQ8nAQgJ7BPYtJo/qowTuQfDCa4ZHIhLjC9frRQh3/UWLrxosG5xWODfYWtpDLKwfmi2gjMV4PIMUdhwZLyMDfZIqR6MAQrB/IQ438iz+1cgU+i8ij7eB5+MeUxcV0ukQhJW/0nwVCm234OqZ+ES3fNPIpWHRo4nq5ZVCdX4ZE3MF+SjZIW2AAu4DFxPpw3tokuOP6z2jNk9AFzjC/WUqlZaIx+6Se5ZeGr4chhEh2IiwChhSUJnGsKtkXHSqTuLZpXf8QZ+ZiRFAAz9XiWxbiOvw6E4+I/0JRutYrALssiRNYBah4I1QzYSU1gIAeMEHz2jvMX9lGGZMfS/uJrv1VtW9UCJMxMCUqgOAA2Hkv95hjyj6toIigG6PyEpzzoJE3ZVqI92F2kWoGSE0l/7aV/sz6jhRl8udbq/Mqu+i9wpbUZqa/ZUCFFi0NLSAQ5s3Le7hPfK1QnMOU8eWkJqiy/XL+remqBwR92Omm8FFANUVzHwOKBsj0Zlrp9o7UW05BJUrUgVXbvJ61r2F+zoAREVSnZt5Tt3JOQs/JRFUway6AvKiQQJihLAOo6AkKiUCTR2G4kbFGiILq4hwgASZGshfdgKRCy+jbHlfDGpNF+vABIwoeTGgkil6kOH/Dg+hNKmqS8N41Y1tQn7i7RkfjMw7gMOQoZcNTKDCNGfgR0gu62ZIkDBIXmea25leCk6VnH2AGcgG4EAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFVzmdAUFVV1YAAAAAAApj+2QAACcQuyA5y12P+HQ9xkG4YvVJJeqDZf4BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZaLZ4aygAAAAIyAxQV////+AAAAABnIBuBAAAAAGcgG4AAAAZXwuHPYAAAAAJwWNtUCsIlij3mTR7FLM4Pu9qzDhJrUtUxIctFWnmj84Af485oCfcURBzjS8v9xlCaHMjofeED+Ml66aUMg3GKE8PDVhr5SAP4MJU436Fr6IFOxCWwq4hIuPuRgtLh6xy3t1dAZmA1SLzhr+OAOS1cKUapaSIeOdv/Mclu2fbSsnRU72f3eNeVU1v13bHKNJ70zxX/fMj109FD2kNQf4+VnjXn0jbxUKWfH5PZBT9oXoD9C59CFRYhLKAuMLSgi1sRBH0T1SmF59vcZjsn""#;
        serde_json::from_str::<VAA>(str).unwrap();
    }

    #[tokio::test]
    async fn fetch_vaa() {
        let url = "https://hermes.pyth.network/api/latest_vaas";

        let client = reqwest::Client::new();

        let response = client
            .get(url)
            .query(&[
                (
                    "ids[]",
                    "c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33",
                ),
                ("binary", "true"),
            ])
            .send()
            .await
            .unwrap();

        let result = response.text().await.unwrap();
        println!("{}", result);
    }

    #[test]
    fn validate_vaa() {
        let storage = &populate_guardian_set();
        let api = &MockApi;

        let block_info = BlockInfo {
            timestamp: Duration::from_nanos(1),
            height: 0,
            hash: Hash256::from_inner([0; 32]),
        };

        let pyth_vaa = PythVaa::from_str(VAA).unwrap();

        pyth_vaa
            .vaa
            .verify(storage, api, block_info, GUARDIAN_SETS)
            .unwrap();
    }
}
