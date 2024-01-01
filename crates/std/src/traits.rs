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

pub trait Storage {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    // minimum bound is always inclusive, maximum bound is always exclusive.
    // if min > max, an empty iterator is to be returned.
    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<Box<dyn Iterator<Item = Record> + 'a>>;

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()>;

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()>;
}
