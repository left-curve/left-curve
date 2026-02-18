use {
    crate::StdResult,
    grug_math::{MathError, MathResult},
};

/// Safely converts input of type T to u32.
/// Errors with a cosmwasm_vm::errors::VmError::ConversionErr if conversion cannot be done.
pub fn to_u32<T>(input: T) -> MathResult<u32>
where
    T: TryInto<u32> + ToString + Copy,
{
    input
        .try_into()
        .map_err(|_| MathError::overflow_conversion::<T, u32>(input))
}

/// Given a compound key consisting of [k1, k2, ..., kN] (N > 1) that is encoded
/// in the following way:
///
/// ```plain
/// len(k1) | k1 | len(k2) | k2 ... len(k{N-1}) | k{N-1} | k{N}
/// ```
///
/// Strip the first key, returns two new byte slices:
///
/// 1. `k1`
/// 2. `len(k2) | k2 ... len(k{N-1}) | k{N-1} | k{N}`
pub fn split_one_key(bytes: &[u8]) -> (&[u8], &[u8]) {
    // Note: This panics if bytes.len() < 2
    let (len_bytes, bytes) = bytes.split_at(2);
    let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]);
    bytes.split_at(len as usize)
}

/// Encodes multiple sections of data into one vector.
///
/// Each section is suffixed by a section length encoded as big endian uint32.
/// Using suffixes instead of prefixes allows reading sections in reverse order,
/// such that the first element does not need to be re-allocated if the contract's
/// data structure supports truncation (such as a Rust vector).
///
/// The resulting data looks like this:
///
/// ```plain
/// section1 | section1_len | section2 | section2_len | section3 | section3_len | …
/// ```
pub fn encode_sections(sections: &[&[u8]]) -> StdResult<Vec<u8>> {
    let mut out_len: usize = sections.iter().map(|section| section.len()).sum();
    out_len += 4 * sections.len();
    let mut out_data = Vec::with_capacity(out_len);
    for section in sections {
        let section_len = to_u32(section.len())?.to_be_bytes();
        out_data.extend_from_slice(section);
        out_data.extend_from_slice(&section_len);
    }
    debug_assert_eq!(out_data.len(), out_len);
    debug_assert_eq!(out_data.capacity(), out_len);
    Ok(out_data)
}

/// Decodes sections of data into multiple slices.
///
/// Each encoded section is suffixed by a section length, encoded as big endian uint32.
///
/// See also: `encode_section`.
pub fn decode_sections(data: &[u8]) -> Vec<&[u8]> {
    let mut result: Vec<&[u8]> = vec![];
    let mut remaining_len = data.len();
    while remaining_len >= 4 {
        let tail_len = u32::from_be_bytes([
            data[remaining_len - 4],
            data[remaining_len - 3],
            data[remaining_len - 2],
            data[remaining_len - 1],
        ]) as usize;
        result.push(&data[remaining_len - 4 - tail_len..remaining_len - 4]);
        remaining_len -= 4 + tail_len;
    }
    debug_assert!(remaining_len == 0);
    result.reverse();
    result
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use crate::{decode_sections, encode_sections};

    #[test]
    fn encode_decode() {
        let data: &[&[u8]] = &[b"this", b"is", b"composite", b"array"];
        let encoded = encode_sections(data).unwrap();
        assert_eq!(data, decode_sections(&encoded));
    }
}
