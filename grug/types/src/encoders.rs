use data_encoding::{BASE64, Encoding, HEXLOWER, HEXUPPER};

/// Describes a scheme for encoding bytes to strings.
pub trait Encoder {
    const NAME: &str;
    const ENCODING: Encoding;
    const PREFIX: &str;
}

/// Binary encoder for raw bytes using the base64 scheme.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Base64Encoder;

impl Encoder for Base64Encoder {
    const ENCODING: Encoding = BASE64;
    const NAME: &str = "Base64";
    const PREFIX: &str = "";
}

/// Binary encoder for raw bytes using the hex scheme.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HexEncoder;

impl Encoder for HexEncoder {
    const ENCODING: Encoding = HEXLOWER;
    const NAME: &str = "Addr32";
    const PREFIX: &str = "";
}

/// Binary encoder for addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddrEncoder;

impl Encoder for AddrEncoder {
    const ENCODING: Encoding = HEXLOWER;
    const NAME: &str = "Addr";
    const PREFIX: &str = "0x";
}

/// Binary encoder for hashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HashEncoder;

impl Encoder for HashEncoder {
    const ENCODING: Encoding = HEXUPPER;
    const NAME: &str = "Hash";
    const PREFIX: &str = "";
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{Addr, Binary, ByteArray, Hash256, Json, JsonDeExt, JsonSerExt, json},
        hex_literal::hex,
        serde::{Serialize, de::DeserializeOwned},
        std::fmt::Debug,
        test_case::test_case,
    };

    #[test_case(
        Binary::from_inner(br#"{"config":{}}"#.to_vec()),
        json!("eyJjb25maWciOnt9fQ=="),
        [
            // Wrong encoding scheme
            json!("7b22636f6e666967223a7b7d7d"),
        ];
        "binary"
    )]
    #[test_case(
        ByteArray::<16>::from_inner(hex!("00112233445566778899aabbccddeeff")),
        json!("ABEiM0RVZneImaq7zN3u/w=="),
        [
            // Too short
            json!("ABEiM0RVZneImaq7zN3u"),
            // Too long
            json!("ABEiM0RVZneImaq7zN3u/wA="),
        ];
        "byte array"
    )]
    #[test_case(
        Addr::from_inner(hex!("299663875422cc5a4574816e6165824d0c5bfdba")),
        json!("0x299663875422cc5a4574816e6165824d0c5bfdba"),
        [
            // Missing prefix
            json!("299663875422cc5a4574816e6165824d0c5bfdb"),
            // Incorrect prefix
            json!("cosmosvaloper1c4k24jzduc365kywrsvf5ujz4ya6mwympnc4en"),
            // Too short
            json!("0x299663875422cc5a4574816e6165824d0c5bfd"),
            // Too long
            json!("0x299663875422cc5a4574816e6165824d0c5bfdbaba"),
        ];
        "address"
    )]
    #[test_case(
        Hash256::from_inner(hex!("299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b")),
        json!("299663875422CC5A4574816E6165824D0C5BFDBA3D58D94D37E8D832A572555B"),
        [
            // Lowercase hex is not accepted
            json!("299663875422cc5a4574816e6165824d0c5bfdba3d58d94d37e8d832a572555b"),
            // Too short
            json!("299663875422CC5A4574816E6165824D0C5BFDBA3D58D94D37E8D832A57255"),
            // Too long
            json!("299663875422CC5A4574816E6165824D0C5BFDBA3D58D94D37E8D832A572555B5B"),
        ];
        "hash"
    )]
    fn encoding_decoding<T, const N: usize>(data: T, encoded: Json, error_cases: [Json; N])
    where
        T: Serialize + DeserializeOwned + Debug + PartialEq,
    {
        // Successful encoding.
        assert_eq!(data.to_json_value().unwrap(), encoded);

        // Successful decoding.
        assert_eq!(encoded.deserialize_json::<T>().unwrap(), data);

        // Unsuccessful decoding.
        for error_case in error_cases {
            assert!(error_case.deserialize_json::<T>().is_err());
        }
    }
}
