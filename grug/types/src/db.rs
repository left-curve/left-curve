use {
    crate::{StdError, StdResult},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::{collections::BTreeMap, fmt},
};

/// A shorthand for an owned KV pair.
pub type Record = (Vec<u8>, Vec<u8>);

/// A batch of Db ops, ready to be committed.
/// For RocksDB, this is similar to rocksdb::WriteBatch.
pub type Batch<K = Vec<u8>, V = Vec<u8>> = BTreeMap<K, Op<V>>;

/// Represents a database operation, either inserting a value or deleting one.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Op<V = Vec<u8>> {
    Insert(V),
    Delete,
}

impl<V> Op<V> {
    // Similar to `Option::as_ref`
    pub fn as_ref(&self) -> Op<&V> {
        match self {
            Op::Insert(v) => Op::Insert(v),
            Op::Delete => Op::Delete,
        }
    }

    // Similar to `Option::map`
    pub fn map<T>(self, f: fn(V) -> T) -> Op<T> {
        match self {
            Op::Insert(v) => Op::Insert(f(v)),
            Op::Delete => Op::Delete,
        }
    }

    pub fn unwrap_value(self) -> V {
        match self {
            Op::Insert(v) => v,
            Op::Delete => panic!("called `Op::unwrap_value()` on a `Delete` value"),
        }
    }

    pub fn into_option(self) -> Option<V> {
        match self {
            Op::Insert(v) => Some(v),
            Op::Delete => None,
        }
    }
}

/// Describing iteration order.
#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Copy, Clone, PartialEq, Eq,
)]
#[borsh(use_discriminant = true)]
pub enum Order {
    Ascending = 1,
    Descending = 2,
}

impl Order {
    pub fn as_str(&self) -> &'static str {
        match self {
            Order::Ascending => "ascending",
            Order::Descending => "descending",
        }
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// We need to convert Order into a primitive type such as `i32` so that it can
// be passed over FFI.
impl From<Order> for i32 {
    fn from(order: Order) -> Self {
        order as _
    }
}

impl TryFrom<i32> for Order {
    type Error = StdError;

    fn try_from(value: i32) -> StdResult<Self> {
        match value {
            1 => Ok(Order::Ascending),
            2 => Ok(Order::Descending),
            _ => {
                let reason = format!("must be 1 (asc) or 2 (desc), found {value}");
                Err(StdError::deserialize::<Self, _, _>("index", reason, value))
            },
        }
    }
}
