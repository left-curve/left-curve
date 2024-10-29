use {
    grug::{Binary, ByteArray, Hash256, HashExt, Inner, StdError, StdResult},
    serde::{de::Visitor, Deserialize},
    std::{collections::BTreeMap, ops::Deref, str::FromStr},
};

#[derive(Debug)]
pub struct GuardianSign {
    pub index: u8,
    pub signature: [u8; VAA::SIGNATURE_LEN - 1],
}

impl GuardianSign {
    pub fn new(data: [u8; VAA::SIGNATURE_LEN]) -> Self {
        Self {
            index: data[0],
            signature: data[1..].try_into().unwrap(),
        }
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
    pub signatures: BTreeMap<u8, ByteArray<{ VAA::SIGNATURE_LEN }>>,
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
                Ok((index, ByteArray::from_inner(signature)))
            })
            .collect::<StdResult<BTreeMap<u8, ByteArray<65>>>>()?;

        // We should use api functions but we are inside a trait, can't use it.
        // For now use the HashExt trait directly.
        // This need double hash
        let hash = bytes.deref().hash256().keccak256().keccak256();

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

pub struct BytesAnalyzer {
    bytes: Vec<u8>,
    index: usize,
}

macro_rules! impl_bytes {
    ($($n:ty => $size:expr),+ ) => {
        paste::paste! {
            $(pub fn [<next_ $n>](&mut self) -> StdResult<$n> {
                if self.index + $size <= self.bytes.len() {
                    let bytes = &self.bytes[self.index..self.index + $size];
                    self.index += $size;
                    Ok(<$n>::from_be_bytes(bytes.try_into()?))
                } else {
                    Err(StdError::host("Not enough bytes".to_string()))
                }
            })*
        }
    };
}

impl BytesAnalyzer {
    impl_bytes!(u16 => 2, u32 => 4, u64 => 8);

    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, index: 0 }
    }

    pub fn next_u8(&mut self) -> u8 {
        self.index += 1;
        self.bytes[self.index - 1]
    }

    pub fn next_bytes<const S: usize>(&mut self) -> StdResult<[u8; S]> {
        if self.index + S <= self.bytes.len() {
            let mut bytes: [u8; S] = [0; S];
            bytes.copy_from_slice(&self.bytes[self.index..self.index + S]);
            self.index += S;
            Ok(bytes)
        } else {
            Err(StdError::host("Not enough bytes".to_string()))
        }
    }

    pub fn consume(mut self) -> Vec<u8> {
        self.bytes.split_off(self.index)
    }
}

impl Deref for BytesAnalyzer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes[self.index..]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

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
}
