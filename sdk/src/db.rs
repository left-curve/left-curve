use crate::Region;

// these are the method that the host must implement
extern "C" {
    fn db_read(key_ptr: u32) -> u32;

    fn db_write(key_ptr: u32, value_ptr: u32);

    fn db_remove(key_ptr: u32);
}

/// A zero-size convenience wrapper around the database imports. Provides more
/// ergonomic functions.
#[derive(Default)]
pub struct Storage;

impl Storage {
    pub fn new() -> Self {
        Self
    }

    pub fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        let key = Region::build(key);
        let key_ptr = &*key as *const Region;

        let value_ptr = unsafe { db_read(key_ptr as u32) };
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
    pub fn write(&mut self, key: &[u8], value: &[u8]) {
        let key = Region::build(key);
        let key_ptr = &*key as *const Region;

        let value = Region::build(value);
        let value_ptr = &*value as *const Region;

        unsafe { db_write(key_ptr as u32, value_ptr as u32) }
    }

    pub fn remove(&mut self, key: &[u8]) {
        let key = Region::build(key);
        let key_ptr = &*key as *const Region;

        unsafe { db_remove(key_ptr as u32) }
    }
}
