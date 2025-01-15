use crate::{Base64Encoder, EncodedBytes, HexEncoder};

/// A fixed length, stack-allocated, base64-encoded byte array.
pub type ByteArray<const N: usize> = EncodedBytes<[u8; N], Base64Encoder>;

/// A variable length, heap-allocated, base64-encoded byte vector.
pub type Binary = EncodedBytes<Vec<u8>, Base64Encoder>;

/// A fixed length, stack-allocated, hex-encoded byte array.
pub type HexByteArray<const N: usize> = EncodedBytes<[u8; N], HexEncoder>;

/// A variable length, heap-allocated, hex-encoded byte vector.
pub type HexBinary = EncodedBytes<Vec<u8>, HexEncoder>;
