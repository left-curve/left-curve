#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
}

pub trait Storage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);

    // minimum bound is always inclusive, maximum bound is always exclusive.
    // if min > max, an empty iterator is to be returned.
    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a>;
}
