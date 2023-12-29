use std::ops::Bound;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
}

pub trait Storage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);

    // NOTE: unlike in cosmwasm, where min is always inclusive and max is always
    // exclusive, here we allow both of them to be either inclusive or exclusive.
    fn scan<'a>(
        &'a self,
        min:   Bound<&[u8]>,
        max:   Bound<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a>;
}
