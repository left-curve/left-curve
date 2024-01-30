use crate::{Batch, Order, Record, Storage};

// a boxed storage is also a storage.
// this is necessary for use in `cw_app::execute::handle_submessage` (see the
// comment there for an explanation)
impl Storage for Box<dyn Storage> {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.as_ref().read(key)
    }

    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        self.as_ref().scan(min, max, order)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.as_mut().write(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.as_mut().remove(key)
    }

    fn flush(&mut self, batch: Batch) {
        self.as_mut().flush(batch)
    }
}
