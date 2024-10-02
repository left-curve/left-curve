use std::borrow::Cow;

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
#[doc(hidden)]
pub fn nested_namespaces_with_key(
    maybe_namespace: Option<&[u8]>,
    prefixes: &[Cow<[u8]>],
    maybe_key: Option<&Cow<[u8]>>,
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
#[doc(hidden)]
pub fn encode_length<B>(bytes: B) -> [u8; 2]
where
    B: AsRef<[u8]>,
{
    let len = bytes.as_ref().len();
    if len > 0xffff {
        panic!(
            "Can't encode length because byte slice is too long: {} > {}",
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
#[doc(hidden)]
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
#[doc(hidden)]
pub fn extend_one_byte(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(0);
    bytes
}

/// Given two byte slices, make a new byte vector that is the two slices joined
/// end to end.
#[doc(hidden)]
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
#[doc(hidden)]
pub fn trim(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    debug_assert!(
        key.starts_with(namespace),
        "byte slice doesn't start with the given namespace"
    );
    key[namespace.len()..].to_vec()
}
