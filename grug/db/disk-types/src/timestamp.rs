use std::{cmp::Ordering, mem};

// --------------------------------- timestamp ---------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U64Timestamp([u8; Self::SIZE]);

impl U64Timestamp {
    pub const SIZE: usize = mem::size_of::<u64>(); // 8
}

impl AsRef<[u8]> for U64Timestamp {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<U64Timestamp> for Vec<u8> {
    fn from(ts: U64Timestamp) -> Self {
        ts.0.to_vec()
    }
}

impl From<u64> for U64Timestamp {
    fn from(ts: u64) -> Self {
        // Note: Use little endian encoding
        Self(ts.to_le_bytes())
    }
}

impl From<&[u8]> for U64Timestamp {
    fn from(bytes: &[u8]) -> Self {
        // Note: Panics if slice is not exactly 8 bytes
        debug_assert_eq!(
            bytes.len(),
            Self::SIZE,
            "incorrect timestamp length: {}, should be {}",
            bytes.len(),
            Self::SIZE
        );
        Self(bytes.try_into().unwrap())
    }
}

impl PartialOrd for U64Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U64Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        // Note: Use little endian encoding
        let a = u64::from_le_bytes(self.0);
        let b = u64::from_le_bytes(other.0);
        a.cmp(&b)
    }
}

// -------------------------------- comparator ---------------------------------

/// Comparator that goes together with `U64Timestamp`.
///
/// This comparator behaves identically to RocksDB's built-in comparator, also
/// using the same name: "leveldb.BytewiseComparator.u64ts".
///
/// Adapted from:
/// - <https://github.com/facebook/rocksdb/blob/main/util/comparator.cc#L238>
/// - <https://github.com/linxGnu/grocksdb/blob/master/db_ts_test.go#L167>
/// - <https://github.com/sei-protocol/sei-db/blob/main/ss/rocksdb/comparator.go>
pub struct U64Comparator;

impl U64Comparator {
    /// Quote from SeiDB:
    /// > We also use the same builtin comparator name so the builtin tools
    /// > `ldb`/`sst_dump` can work with the database.
    pub const NAME: &'static str = "leveldb.BytewiseComparator.u64ts";

    /// Compares two internal keys with timestamp suffix, larger timestamp
    /// comes first.
    pub fn compare(a: &[u8], b: &[u8]) -> Ordering {
        // First, compare the keys without timestamps. If the keys are different
        // then we don't have to consider timestamps at all.
        let ord = Self::compare_without_ts(a, true, b, true);
        if ord != Ordering::Equal {
            return ord;
        }

        // The keys are the same, now we compare the timestamps.
        // The larger (newer) timestamp should come first, meaning seek operation
        // will try to find a version less than or equal to the target version.
        Self::compare_ts(
            extract_timestamp_from_user_key(a),
            extract_timestamp_from_user_key(b),
        )
        .reverse()
    }

    /// Compares timestamps as little endian encoded integers.
    pub fn compare_ts(bz1: &[u8], bz2: &[u8]) -> Ordering {
        let ts1 = U64Timestamp::from(bz1);
        let ts2 = U64Timestamp::from(bz2);
        ts1.cmp(&ts2)
    }

    // Compares two internal keys without the timestamp part.
    pub fn compare_without_ts(
        mut a: &[u8],
        a_has_ts: bool,
        mut b: &[u8],
        b_has_ts: bool,
    ) -> Ordering {
        if a_has_ts {
            a = strip_timestamp_from_user_key(a);
        }
        if b_has_ts {
            b = strip_timestamp_from_user_key(b);
        }
        a.cmp(b)
    }
}

#[inline]
fn extract_timestamp_from_user_key(key: &[u8]) -> &[u8] {
    &key[(key.len() - U64Timestamp::SIZE)..]
}

#[inline]
fn strip_timestamp_from_user_key(key: &[u8]) -> &[u8] {
    &key[..(key.len() - U64Timestamp::SIZE)]
}
