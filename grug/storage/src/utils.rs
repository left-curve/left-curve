use {crate::RawKey, grug_types::encode_length};

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
    prefixes: &[RawKey],
    maybe_key: Option<RawKey>,
) -> Vec<u8> {
    let mut size = 0;
    if let Some(namespace) = maybe_namespace {
        size += namespace.len() + 2;
    }
    for prefix in prefixes {
        size += prefix.as_ref().len() + 2;
    }
    if let Some(key) = &maybe_key {
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
