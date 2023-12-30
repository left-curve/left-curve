use crate::{Order, Region, Storage};

// these are the method that the host must implement
extern "C" {
    fn db_read(key_ptr: usize) -> usize;

    fn db_write(key_ptr: usize, value_ptr: usize);

    fn db_remove(key_ptr: usize);

    fn db_scan(min_ptr: usize, max_ptr: usize, order: i32) -> u32;

    fn db_next(iterator_id: u32) -> u32;
}

/// A zero-size convenience wrapper around the database imports. Provides more
/// ergonomic functions.
pub struct ExternalStorage;

impl Storage for ExternalStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        let key = Region::build(key);
        let key_ptr = &*key as *const Region;

        let value_ptr = unsafe { db_read(key_ptr as usize) };
        if value_ptr == 0 {
            // we interpret a zero pointer as meaning the key doesn't exist
            return None;
        }

        unsafe { Some(Region::consume(value_ptr as *mut Region)) }
        // NOTE: key_ptr goes out of scope here, so the Region is dropped.
        // however, `key` is NOT dropped, since we're only working with a
        // borrowed reference here.
        // same case with `write` and `remove`.
    }

    // note: cosmwasm doesn't allow empty values:
    // https://github.com/CosmWasm/cosmwasm/blob/v1.5.0/packages/std/src/imports.rs#L111
    // this is because its DB backend doesn't distinguish between an empty value
    // vs a non-existent value. but this isn't a problem for us.
    fn write(&mut self, key: &[u8], value: &[u8]) {
        let key = Region::build(key);
        let key_ptr = &*key as *const Region;

        let value = Region::build(value);
        let value_ptr = &*value as *const Region;

        unsafe { db_write(key_ptr as usize, value_ptr as usize) }
    }

    fn remove(&mut self, key: &[u8]) {
        let key = Region::build(key);
        let key_ptr = &*key as *const Region;

        unsafe { db_remove(key_ptr as usize) }
    }

    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let min_ptr = build_optional_region(min);
        let max_ptr = build_optional_region(max);

        let iterator_id = unsafe { db_scan(min_ptr, max_ptr, order.into()) };
        let iterator = ExternalIterator { iterator_id };

        Box::new(iterator)
    }
}

pub struct ExternalIterator {
    iterator_id: u32,
}

impl Iterator for ExternalIterator {
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        let ptr = unsafe { db_next(self.iterator_id) };

        // the host returning a zero pointer means iteration has finished
        if ptr == 0 {
            return None;
        }

        unsafe { Some(split_tail(Region::consume(ptr as *mut Region))) }
    }
}

// this is the same as Region::build but the data is optional. also, it casts
// the pointer to usize. if the data is None, return zero.
fn build_optional_region(maybe_data: Option<&[u8]>) -> usize {
    // a zero memory address tells the host that no data has been loaded into
    // memory. in case of db_scan, it means the bound is None.
    let Some(data) = maybe_data else {
        return 0;
    };

    let region = Region::build(data);
    let ptr = &*region as *const Region;

    ptr as usize
}

// unlike storage keys in Map, where we prefix the length, like this:
// storage_key := len(namespace) | namespace | len(k1) | k1 | len(k2) | k2 | k3
//
// here, when the host loads the next value into Wasm memory, we do it like this:
// data := key | value | len(key)
//
// this is because in this way, we can simply pop out the key without having to
// allocate a new Vec.
//
// another difference from cosmwasm is we use 2 bytes (instead of 4) for the
// length. this means the key cannot be more than u16::MAX = 65535 bytes long,
// which is always true is practice (we set max key length in Item and Map).
fn split_tail(mut data: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    // pop two bytes from the end, must both be Some
    let (Some(byte1), Some(byte2)) = (data.pop(), data.pop()) else {
        panic!("[ExternalIterator]: can't read length suffix");
    };

    // note the order here between the two bytes
    let key_len = u16::from_be_bytes([byte2, byte1]);
    let value = data.split_off(key_len.into());

    (data, value)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn spliting_tail() {
        let key = b"foobar";
        let value = b"fuzzbuzz";

        let mut data = Vec::with_capacity(key.len() + value.len() + 2);
        data.extend_from_slice(key);
        data.extend_from_slice(value);
        data.extend_from_slice(&(key.len() as u16).to_be_bytes());

        assert_eq!((key.to_vec(), value.to_vec()), split_tail(data))
    }
}
