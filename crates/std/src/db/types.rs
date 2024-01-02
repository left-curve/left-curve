use {anyhow::anyhow, std::collections::BTreeMap};

/// A shorthand for an owned KV pair.
pub type Record = (Vec<u8>, Vec<u8>);

/// A batch of Db ops, ready to be committed.
/// For RocksDB, this is similar to rocksdb::WriteBatch.
pub type Batch = BTreeMap<Vec<u8>, Op>;

/// Represents a database operation, either inserting a value or deleting one.
#[derive(Debug, Clone)]
pub enum Op {
    Put(Vec<u8>),
    Delete,
}

/// Describing iteration order.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Order {
    Ascending = 1,
    Descending = 2,
}

// we need to convert Order into a primitive type such as i32 so that it can be
// passed over FFI
impl From<Order> for i32 {
    fn from(order: Order) -> Self {
        order as _
    }
}

impl TryFrom<i32> for Order {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> anyhow::Result<Self> {
        match value {
            1 => Ok(Order::Ascending),
            2 => Ok(Order::Descending),
            _ => Err(anyhow!("Invalid iterator order {value}, must be 1 (asc) or 2 (desc)")),
        }
    }
}
