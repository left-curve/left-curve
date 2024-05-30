/// Combine a namespace a one or more keys into a full byte path.
///
/// The namespace and all keys other than the last one is prefixed with
/// their lengths (2 bytes big-endian). This helps us know where a key ends
/// and where the next key starts.
///
/// E.g. if keys are [key1, key2, key3], the resulting byte path is:
/// len(namespace) | namespace | len(key1) | key1 | len(key2) | key2 | key3
///
/// Panics if any key's length exceeds u16::MAX (because we need to put the
/// length into 2 bytes)
pub fn nested_namespaces_with_key(
    maybe_namespace: Option<&[u8]>,
    prefixes: &[impl AsRef<[u8]>],
    maybe_key: Option<&impl AsRef<[u8]>>,
) -> Vec<u8> {
    let mut size = 0;
    if let Some(namespace) = maybe_namespace {
        size += namespace.len() + 2;
    }
    for prefix in prefixes {
        size += prefix.as_ref().len() + 2;
    }
    if let Some(key) = maybe_key {
        size += key.as_ref().len();
    }

    let mut out = Vec::with_capacity(size);
    if let Some(namespace) = maybe_namespace {
        out.extend_from_slice(&encode_length(namespace));
        out.extend_from_slice(namespace);
    }
    for prefix in prefixes {
        out.extend_from_slice(&encode_length(prefix));
        out.extend_from_slice(prefix.as_ref());
    }
    if let Some(key) = maybe_key {
        out.extend_from_slice(key.as_ref());
    }
    out
}

/// Given a byte slice, return two bytes in big endian representing its length.
/// Panic if the given byte slice is longer than the biggest length that can be
/// represented by a two bytes (i.e. 65535).
pub fn encode_length(bytes: impl AsRef<[u8]>) -> [u8; 2] {
    let len = bytes.as_ref().len();
    if len > 0xffff {
        panic!(
            "Can't encode length becayse byte slice is too long: {} > {}",
            len,
            u16::MAX
        );
    }

    (bytes.as_ref().len() as u16).to_be_bytes()
}

// NOTE: this doesn't work if the bytes are entirely 255.
// in practice, the input bytes is a length-prefixed Map namespace. for the
// bytes to be entirely 255, the namespace must be u16::MAX = 65535 byte long
// (so that the two prefixed length bytes are [255, 255]).
// we can prevent this by introducing a max length for the namespace.
// assert this max length at compile time when the user calls Map::new.
pub fn increment_last_byte(mut bytes: Vec<u8>) -> Vec<u8> {
    debug_assert!(
        bytes.iter().any(|x| *x != u8::MAX),
        "bytes are entirely 255"
    );
    for byte in bytes.iter_mut().rev() {
        if *byte == u8::MAX {
            *byte = 0;
        } else {
            *byte += 1;
            break;
        }
    }
    bytes
}

/// Given an extendable byte slice, append a zero byte to the end of it.
/// This is useful for dealing with iterator bounds.
pub fn extend_one_byte(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(0);
    bytes
}

/// Given two byte slices, make a new byte vector that is the two slices joined
/// end to end.
pub fn concat(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(namespace.len() + key.len());
    out.extend_from_slice(namespace);
    out.extend_from_slice(key);
    out
}

/// Given a byte slice that is prefixed with a namespace, trim the namespace,
/// return the suffix. The reverse of what `concat` function does.
///
/// Note that this function only checks whether the byte slice is actually
/// prefixed with the namespace in debug mode. In release we skip this for
/// performance. You must make sure only use this function we you're sure the
/// slice actually is prefixed with the namespace.
pub fn trim(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    debug_assert!(
        key.starts_with(namespace),
        "byte slice doesn't start with the given namespace"
    );
    key[namespace.len()..].to_vec()
}

/// Given a compound key consisting of [k1, k2, ..., kN] (N > 1) that is encoded
/// in the following way:
///
/// len(k1) | k1 | len(k2) | k2 ... len(k{N-1}) | k{N-1} | k{N}
///
/// Strip the first key, returns two new byte slices:
/// 1. k1
/// 2. len(k2) | k2 ... len(k{N-1}) | k{N-1} | k{N}
pub fn split_one_key(bytes: &[u8]) -> (&[u8], &[u8]) {
    // NOTE: this panics if bytes.len() < 2
    let (len_bytes, bytes) = bytes.split_at(2);
    // this unwrap can't fail since split at position 2
    let len = u16::from_be_bytes(len_bytes.try_into().unwrap());
    bytes.split_at(len as usize)
}
