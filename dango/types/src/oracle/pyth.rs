use {
    super::{GuardianSetInfo, WormholeVAA},
    anyhow::bail,
    byteorder::BigEndian,
    grug::{AddrEncoder, Api, Binary, BlockInfo, EncodedBytes, Inner, Map, Storage},
    pyth_sdk::{Price, PriceFeed, PriceIdentifier},
    pyth_wormhole::{BatchPriceAttestation, PriceStatus},
    pythnet_sdk::{
        accumulators::merkle::MerkleRoot,
        hashers::keccak256_160::Keccak160,
        messages::Message,
        wire::{
            from_slice,
            v1::{AccumulatorUpdateData, Proof, WormholeMessage, WormholePayload},
        },
    },
    serde::{de::Visitor, Deserialize},
    std::str::FromStr,
};

pub const PYTHNET_ACCUMULATOR_UPDATE_MAGIC: &[u8; 4] = b"PNAU";

macro_rules! uncast_enum {
    ($data:expr, $name:path, $($params:ident),*) => {
        match $data {
            $name { $($params),* } => {
                ($($params),*)
            },
        }
    };
    ($data:expr, $name:path) => {
        match $data {
            $name(t) => {
                t
            },
        }
    };
}

pub type PythId = EncodedBytes<[u8; 32], AddrEncoder>;

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct PythVaa {
    vaa: WormholeVAA,
    feeds: Vec<PriceFeed>,
}

impl PythVaa {
    pub fn verify(
        self,
        storage: &dyn Storage,
        api: &dyn Api,
        block: BlockInfo,
        guardian_set: Map<u32, GuardianSetInfo>,
    ) -> anyhow::Result<Vec<PriceFeed>> {
        self.vaa.verify(storage, api, block, guardian_set)?;
        Ok(self.feeds)
    }

    pub fn new<T>(bytes: T) -> anyhow::Result<Self>
    where
        T: Into<Vec<u8>>,
    {
        let bytes = bytes.into();

        let (vaa, feeds) = if bytes[0..4] == *PYTHNET_ACCUMULATOR_UPDATE_MAGIC {
            let res = AccumulatorUpdateData::try_from_slice(&bytes)?;

            let (vaa, updates) = uncast_enum!(res.proof, Proof::WormholeMerkle, vaa, updates);

            let parsed_vaa = WormholeVAA::new(Vec::from(vaa))?;
            let msg = WormholeMessage::try_from_bytes(parsed_vaa.payload.clone())?;

            let root = MerkleRoot::<Keccak160>::new(
                uncast_enum!(msg.payload, WormholePayload::Merkle).root,
            );

            let feeds = updates
                .into_iter()
                .map(|update| {
                    let message_vec = Vec::from(update.message);

                    if !root.check(update.proof, &message_vec) {
                        bail!("invalid proof");
                    }

                    let msg = from_slice::<BigEndian, Message>(&message_vec)?;

                    let price_feed = match msg {
                        Message::PriceFeedMessage(price_feed_message) => PriceFeed::new(
                            PriceIdentifier::new(price_feed_message.feed_id),
                            Price {
                                price: price_feed_message.price,
                                conf: price_feed_message.conf,
                                expo: price_feed_message.exponent,
                                publish_time: price_feed_message.publish_time,
                            },
                            Price {
                                price: price_feed_message.ema_price,
                                conf: price_feed_message.ema_conf,
                                expo: price_feed_message.exponent,
                                publish_time: price_feed_message.publish_time,
                            },
                        ),
                        _ => bail!("invalid message"),
                    };

                    Ok(price_feed)
                })
                .collect::<anyhow::Result<Vec<_>>>()?;

            (parsed_vaa, feeds)
        } else {
            let vaa = WormholeVAA::new(bytes)?;
            let batch_attestation =
                BatchPriceAttestation::deserialize(&vaa.payload[..]).map_err(|err| {
                    anyhow::anyhow!("Failed to deserialize batch attestation: {:?}", err)
                })?;

            let feeds: Vec<_> = batch_attestation
                .price_attestations
                .into_iter()
                .map(|price_attestation| match price_attestation.status {
                    PriceStatus::Trading => PriceFeed::new(
                        PriceIdentifier::new(price_attestation.price_id.to_bytes()),
                        Price {
                            price: price_attestation.price,
                            conf: price_attestation.conf,
                            expo: price_attestation.expo,
                            publish_time: price_attestation.publish_time,
                        },
                        Price {
                            price: price_attestation.ema_price,
                            conf: price_attestation.ema_conf,
                            expo: price_attestation.expo,
                            publish_time: price_attestation.publish_time,
                        },
                    ),
                    _ => PriceFeed::new(
                        PriceIdentifier::new(price_attestation.price_id.to_bytes()),
                        Price {
                            price: price_attestation.prev_price,
                            conf: price_attestation.prev_conf,
                            expo: price_attestation.expo,
                            publish_time: price_attestation.prev_publish_time,
                        },
                        Price {
                            price: price_attestation.ema_price,
                            conf: price_attestation.ema_conf,
                            expo: price_attestation.expo,
                            publish_time: price_attestation.prev_publish_time,
                        },
                    ),
                })
                .collect();

            (vaa, feeds)
        };

        Ok(PythVaa { vaa, feeds })
    }
}

impl FromStr for PythVaa {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        PythVaa::new(Binary::from_str(s)?.into_inner())
    }
}

pub struct PythVAAVisitor;

impl<'de> Visitor<'de> for PythVAAVisitor {
    type Value = PythVaa;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("pyth-vaa")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        PythVaa::from_str(v).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for PythVaa {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(PythVAAVisitor)
    }
}

#[cfg(test)]
mod tests {

    use grug::{Duration, Hash256, MockApi};

    use crate::oracle::tests::{populate_guardian_set, GUARDIAN_SETS, VAA};

    use super::*;

    #[test]
    fn des_pyth_vaa() {
        let str = r#""UE5BVQEAAAADuAEAAAAEDQBkMyJzGWOwAlhd3NDvcYJvct5KACRi6oi9InIE/PYqXh1z92MOXFyFPGP5y9uOpubgMIvUh/pa5aXsM/z+aaCdAALKQlwSVB5YIQ/C0NuqXqam0fAAQYUJeBe+G7rjnv7UXhHRIqNiqCvTE1ygz3zUztg07pqoYahCI7SlqI23hHizAAPG7cQdoENAUMDgYC1znnRkG8NUDS/Yzlxb3Krl/fKDUjpgKM2ZEB5HD11bCTzIhPHTI8KQxIDbyKxF6o4cwf5QAAQxrIWXQX0Bx9/lDEDfFOOqRU6LwZhFMmiDwUedUxsIvR73V/yfZKNtObHA0O9McjdTo1JibRqnbNqw6H8hw4/JAAax4DOJ/M8yxbIk88rV0n8sttzelXPuMnnJCXV2CFpwlSqYu0cQ+gmWvfjK/zJSFKHhNF0N7wzOX9J/bghUeQ8nAQgJ7BPYtJo/qowTuQfDCa4ZHIhLjC9frRQh3/UWLrxosG5xWODfYWtpDLKwfmi2gjMV4PIMUdhwZLyMDfZIqR6MAQrB/IQ438iz+1cgU+i8ij7eB5+MeUxcV0ukQhJW/0nwVCm234OqZ+ES3fNPIpWHRo4nq5ZVCdX4ZE3MF+SjZIW2AAu4DFxPpw3tokuOP6z2jNk9AFzjC/WUqlZaIx+6Se5ZeGr4chhEh2IiwChhSUJnGsKtkXHSqTuLZpXf8QZ+ZiRFAAz9XiWxbiOvw6E4+I/0JRutYrALssiRNYBah4I1QzYSU1gIAeMEHz2jvMX9lGGZMfS/uJrv1VtW9UCJMxMCUqgOAA2Hkv95hjyj6toIigG6PyEpzzoJE3ZVqI92F2kWoGSE0l/7aV/sz6jhRl8udbq/Mqu+i9wpbUZqa/ZUCFFi0NLSAQ5s3Le7hPfK1QnMOU8eWkJqiy/XL+remqBwR92Omm8FFANUVzHwOKBsj0Zlrp9o7UW05BJUrUgVXbvJ61r2F+zoAREVSnZt5Tt3JOQs/JRFUway6AvKiQQJihLAOo6AkKiUCTR2G4kbFGiILq4hwgASZGshfdgKRCy+jbHlfDGpNF+vABIwoeTGgkil6kOH/Dg+hNKmqS8N41Y1tQn7i7RkfjMw7gMOQoZcNTKDCNGfgR0gu62ZIkDBIXmea25leCk6VnH2AGcgG4EAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFVzmdAUFVV1YAAAAAAApj+2QAACcQuyA5y12P+HQ9xkG4YvVJJeqDZf4BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZaLZ4aygAAAAIyAxQV////+AAAAABnIBuBAAAAAGcgG4AAAAZXwuHPYAAAAAJwWNtUCsIlij3mTR7FLM4Pu9qzDhJrUtUxIctFWnmj84Af485oCfcURBzjS8v9xlCaHMjofeED+Ml66aUMg3GKE8PDVhr5SAP4MJU436Fr6IFOxCWwq4hIuPuRgtLh6xy3t1dAZmA1SLzhr+OAOS1cKUapaSIeOdv/Mclu2fbSsnRU72f3eNeVU1v13bHKNJ70zxX/fMj109FD2kNQf4+VnjXn0jbxUKWfH5PZBT9oXoD9C59CFRYhLKAuMLSgi1sRBH0T1SmF59vcZjsn""#;
        serde_json::from_str::<PythVaa>(str).unwrap();
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