use crate::RawKey;

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
pub(super) fn nested_namespaces_with_key(
    maybe_namespace: Option<&[u8]>,
    prefixes:        &[RawKey],
    maybe_key:       Option<&RawKey>,
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

fn encode_length(bytes: impl AsRef<[u8]>) -> [u8; 2] {
    let len = bytes.as_ref().len();
    if len > 0xffff {
        panic!("Can't encode length becayse byte slice is too long: {} > {}", len, u16::MAX);
    }

    (bytes.as_ref().len() as u16).to_be_bytes()
}

// NOTE: this doesn't work if the bytes are entirely 255.
// in practice, the input bytes is a length-prefixed Map namespace. for the
// bytes to be entirely 255, the namespace must be u16::MAX = 65535 byte long
// (so that the two prefixed length bytes are [255, 255]).
// we can prevent this by introducing a max length for the namespace.
// assert this max length at compile time when the user calls Map::new.
pub(super) fn increment_last_byte(mut bytes: Vec<u8>) -> Vec<u8> {
    debug_assert!(bytes.iter().any(|x| *x != u8::MAX), "[Map]: Namespace is entirely 255");
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

pub(super) fn extend_one_byte(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(0);
    bytes
}

pub(super) fn concat(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    let mut out = namespace.to_vec();
    out.extend_from_slice(key);
    out
}

pub(super) fn trim(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    key[namespace.len()..].to_vec()
}

pub(super) fn split_one_key(bytes: &[u8]) -> anyhow::Result<(&[u8], &[u8])> {
    let (len_bytes, bytes) = bytes.split_at(2);
    let len = u16::from_be_bytes(len_bytes.try_into()?);
    Ok(bytes.split_at(len as usize))
}
