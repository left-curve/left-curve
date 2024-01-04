use anyhow::anyhow;

/// A shorthand for an owned KV pair.
pub type Record = (Vec<u8>, Vec<u8>);

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

/// Describing a KV store that supports read, write, and iteration.
///
/// A question you may have is why these methods are not fallible (that is, for
/// example, why `read` returns an Option<Vec<u8>> instead of a Result<Option<Vec<u8>>>).
/// Surely reading/writing a database may fail?
///
/// The answer is that this trait describe the KV store _viewed from the Wasm
/// module's perspective_. Indeed DB reads/writes may fail, but if they fail,
/// the contract call is aborted in the host function; the Wasm module never
/// receives a response. As long as the Wasm module receives a response, the
/// read/write must have been successful.
pub trait Storage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Iterate over data in the KV store under the given bounds and order.
    /// Minimum bound is inclusive, maximum bound is exclusive.
    /// If min > max, an empty iterator is to be returned.
    ///
    /// NOTE: Rust's BTreeMap panics if max > max. We don't want this behavior.
    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);
}
