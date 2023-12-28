pub trait Storage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);

    // min is inclusive, max is exclusive. iteration order is ascending.
    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
    ) -> Box<dyn Iterator<Item = Record> + 'a>;
}

// (key, value)
pub type Record = (Vec<u8>, Vec<u8>);
