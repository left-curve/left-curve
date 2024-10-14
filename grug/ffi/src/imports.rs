use {
    crate::Region,
    grug_types::{
        encode_sections, Addr, Api, BorshDeExt, BorshSerExt, GenericResult, Order, Querier, Query,
        QueryResponse, Record, StdError, StdResult, Storage, VerificationError,
    },
};

// These are the method that the host must implement.
// We use `usize` to denote memory addresses, and `i32` to denote other data.
extern "C" {
    // Database operations.
    //
    // Note that these methods are not fallible. the reason is that if a DB op
    // indeed fails, the host should have thrown an error and kill the execution.
    // from the Wasm module's perspective, if a response is received, the DB op
    // must have succeeded.
    fn db_read(key_ptr: usize) -> usize;
    fn db_scan(min_ptr: usize, max_ptr: usize, order: i32) -> i32;
    fn db_next(iterator_id: i32) -> usize;
    fn db_next_key(iterator_id: i32) -> usize;
    fn db_next_value(iterator_id: i32) -> usize;
    fn db_write(key_ptr: usize, value_ptr: usize);
    fn db_remove(key_ptr: usize);
    fn db_remove_range(min_ptr: usize, max_ptr: usize);

    // Signature verification
    // Return value of 0 means ok; any value other than 0 means error.
    fn secp256r1_verify(msg_hash_ptr: usize, sig_ptr: usize, pk_ptr: usize) -> u32;
    fn secp256k1_verify(msg_hash_ptr: usize, sig_ptr: usize, pk_ptr: usize) -> u32;
    fn secp256k1_pubkey_recover(
        msg_hash_ptr: usize,
        sig_ptr: usize,
        recovery_id: u8,
        compressed: u8,
    ) -> u64;
    fn ed25519_verify(msg_hash_ptr: usize, sig_ptr: usize, pk_ptr: usize) -> u32;
    fn ed25519_batch_verify(prehash_msgs_ptr: usize, sigs_ptr: usize, pks_ptr: usize) -> u32;

    // Hashes
    fn sha2_256(data_ptr: usize) -> usize;
    fn sha2_512(data_ptr: usize) -> usize;
    fn sha2_512_truncated(data_ptr: usize) -> usize;
    fn sha3_256(data_ptr: usize) -> usize;
    fn sha3_512(data_ptr: usize) -> usize;
    fn sha3_512_truncated(data_ptr: usize) -> usize;
    fn keccak256(data_ptr: usize) -> usize;
    fn blake2s_256(data_ptr: usize) -> usize;
    fn blake2b_512(data_ptr: usize) -> usize;
    fn blake3(data_ptr: usize) -> usize;

    // Print a debug message to the client's CLI output.
    fn debug(addr_ptr: usize, msg_ptr: usize);

    // Send a query request to the chain.
    // Not to be confused with the `query` export.
    fn query_chain(req: usize) -> usize;
}

// ---------------------------------- storage ----------------------------------

/// A zero-sized wrapper over database-related FFI fucntions.
#[derive(Clone)]
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
        // NOTE: `key_ptr` goes out of scope here, so the `Region` is dropped.
        // However, `key` is _not_ dropped, since we're only working with a
        // borrowed reference here.
        // Same case with `write` and `remove`.
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let iterator_id = unsafe { register_iterator(min, max, order) };
        Box::new(ExternalIterator { iterator_id })
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let iterator_id = unsafe { register_iterator(min, max, order) };
        Box::new(ExternalPartialIterator {
            iterator_id,
            is_keys: true,
        })
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let iterator_id = unsafe { register_iterator(min, max, order) };
        Box::new(ExternalPartialIterator {
            iterator_id,
            is_keys: false,
        })
    }

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

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        let min_region = min.map(Region::build);
        let min_ptr = get_optional_region_ptr(min_region.as_ref());

        let max_region = max.map(Region::build);
        let max_ptr = get_optional_region_ptr(max_region.as_ref());

        unsafe { db_remove_range(min_ptr, max_ptr) }
    }
}

/// Iterator wrapper over the `db_next` import, which iterates over both the
/// raw keys and raw values.
pub struct ExternalIterator {
    iterator_id: i32,
}

impl Iterator for ExternalIterator {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        let ptr = unsafe { db_next(self.iterator_id) };

        // The host returning a zero pointer means iteration has reached end.
        if ptr == 0 {
            return None;
        }

        unsafe { Some(split_tail(Region::consume(ptr as *mut Region))) }
    }
}

/// Iterator wrapper over either the `db_next_key` or `db_next_value` imports,
/// which iterates over either only the raw keys, or only the raw values.
pub struct ExternalPartialIterator {
    iterator_id: i32,
    /// If `true`, the iterator uses the `db_next_key` to iterate over raw keys;
    /// otherwise, it uses `db_next_value` to iterate over raw values.
    is_keys: bool,
}

impl Iterator for ExternalPartialIterator {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let ptr = unsafe {
            if self.is_keys {
                db_next_key(self.iterator_id)
            } else {
                db_next_value(self.iterator_id)
            }
        };

        // The host returning a zero pointer means iteration has reached end.
        if ptr == 0 {
            return None;
        }

        unsafe { Some(Region::consume(ptr as *mut Region)) }
    }
}

unsafe fn register_iterator(min: Option<&[u8]>, max: Option<&[u8]>, order: Order) -> i32 {
    // IMPORTANT: We must to keep the `Region`s in scope until end of the func.
    // Make sure to use `as_ref` so that the `Region`s don't get consumed.
    let min_region = min.map(Region::build);
    let min_ptr = get_optional_region_ptr(min_region.as_ref());

    let max_region = max.map(Region::build);
    let max_ptr = get_optional_region_ptr(max_region.as_ref());

    db_scan(min_ptr, max_ptr, order.into())
}

// Clippy has a false positive here. We _have_ to take `Option<&Box<Region>>`,
// not `Option<&Region>`.
#[allow(clippy::borrowed_box)]
fn get_optional_region_ptr(maybe_region: Option<&Box<Region>>) -> usize {
    // A zero memory address tells the host that no data has been loaded into
    // memory. In case of `db_scan`, it means the bound is `None`.
    let Some(region) = maybe_region else {
        return 0;
    };

    (region.as_ref() as *const Region) as usize
}

// Unlike storage keys in `Map`, where we prefix the length, like this:
//
// storage_key := len(namespace) | namespace | len(k1) | k1 | len(k2) | k2 | k3
//
// Here, when the host loads the next value into Wasm memory, we do it like this:
//
// data := key | value | len(key)
//
// This is because in this way, we can simply pop out the key without having to
// allocate a new `Vec`.
//
// Another difference from CosmWasm is we use 2 bytes (instead of 4) for the
// length. This means the key cannot be more than `u16::MAX` = 65535 bytes long,
// which is always true is practice (we set max key length in `Item` and `Map`).
#[inline]
fn split_tail(mut data: Vec<u8>) -> Record {
    // Pop two bytes from the end, must both be Some
    let (Some(byte1), Some(byte2)) = (data.pop(), data.pop()) else {
        panic!("[ExternalIterator]: can't read length suffix");
    };

    // Note the order here between the two bytes
    let key_len = u16::from_be_bytes([byte2, byte1]);
    let value = data.split_off(key_len.into());

    (data, value)
}

// ------------------------------------ api ------------------------------------

/// A zero-sized wrapper over cryptography-related (signature verification and
/// hashing) FFI fucntions.
pub struct ExternalApi;

macro_rules! impl_hash_method {
    ($name:ident, $len:literal) => {
        fn $name(&self, data: &[u8]) -> [u8; $len] {
            let data_region = Region::build(data);
            let data_ptr = &*data_region as *const Region;

            let hash_region = unsafe {
                let hash_ptr = $name(data_ptr as usize);
                Region::consume(hash_ptr as *mut Region)
            };

            // We trust the host returns a hash of the correct length, therefore
            // unwrapping it here safely.
            hash_region.try_into().unwrap()
        }
    };
}

impl Api for ExternalApi {
    impl_hash_method!(sha2_256, 32);

    impl_hash_method!(sha2_512, 64);

    impl_hash_method!(sha2_512_truncated, 32);

    impl_hash_method!(sha3_256, 32);

    impl_hash_method!(sha3_512, 64);

    impl_hash_method!(sha3_512_truncated, 32);

    impl_hash_method!(keccak256, 32);

    impl_hash_method!(blake2s_256, 32);

    impl_hash_method!(blake2b_512, 64);

    impl_hash_method!(blake3, 32);

    fn debug(&self, addr: Addr, msg: &str) {
        let addr_region = Region::build(&addr);
        let addr_ptr = &*addr_region as *const Region;
        let msg_region = Region::build(msg.as_bytes());
        let msg_ptr = &*msg_region as *const Region;

        unsafe { debug(addr_ptr as usize, msg_ptr as usize) }
    }

    fn secp256r1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        let msg_hash_region = Region::build(msg_hash);
        let msg_hash_ptr = &*msg_hash_region as *const Region;

        let sig_region = Region::build(sig);
        let sig_ptr = &*sig_region as *const Region;

        let pk_region = Region::build(pk);
        let pk_ptr = &*pk_region as *const Region;

        let return_value =
            unsafe { secp256r1_verify(msg_hash_ptr as usize, sig_ptr as usize, pk_ptr as usize) };

        if return_value == 0 {
            Ok(())
        } else {
            Err(VerificationError::from_error_code(return_value).into())
        }
    }

    fn secp256k1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        let msg_hash_region = Region::build(msg_hash);
        let msg_hash_ptr = &*msg_hash_region as *const Region;

        let sig_region = Region::build(sig);
        let sig_ptr = &*sig_region as *const Region;

        let pk_region = Region::build(pk);
        let pk_ptr = &*pk_region as *const Region;

        let return_value =
            unsafe { secp256k1_verify(msg_hash_ptr as usize, sig_ptr as usize, pk_ptr as usize) };

        if return_value == 0 {
            Ok(())
        } else {
            Err(VerificationError::from_error_code(return_value).into())
        }
    }

    fn secp256k1_pubkey_recover(
        &self,
        msg_hash: &[u8],
        sig: &[u8],
        recovery_id: u8,
        compressed: bool,
    ) -> StdResult<Vec<u8>> {
        let msg_hash_region = Region::build(msg_hash);
        let msg_hash_ptr = &*msg_hash_region as *const Region;

        let sig_region = Region::build(sig);
        let sig_ptr = &*sig_region as *const Region;

        let return_value = unsafe {
            secp256k1_pubkey_recover(
                msg_hash_ptr as usize,
                sig_ptr as usize,
                recovery_id,
                compressed as u8,
            )
        };

        let error_code = (return_value >> 32) as u32;
        let pk_ptr = return_value as u32;

        // Sanity check: if the host works correctly, then one and only one
        // between `error_code` and `pk_ptr` is zero.
        // Here we efficiently assert this using the XOR operator (`^`).
        debug_assert!(
            (error_code == 0) ^ (pk_ptr == 0),
            "host returned invalid response for `secp256k1_pubkey_recover`! error_code: {error_code}, pk_ptr: {pk_ptr}"
        );

        if error_code == 0 {
            let pk = unsafe { Region::consume(pk_ptr as *mut Region) };
            Ok(pk)
        } else {
            Err(VerificationError::from_error_code(error_code).into())
        }
    }

    fn ed25519_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        let msg_hash_region = Region::build(msg_hash);
        let msg_hash_ptr = &*msg_hash_region as *const Region;

        let sig_region = Region::build(sig);
        let sig_ptr = &*sig_region as *const Region;

        let pk_region = Region::build(pk);
        let pk_ptr = &*pk_region as *const Region;

        let return_value =
            unsafe { ed25519_verify(msg_hash_ptr as usize, sig_ptr as usize, pk_ptr as usize) };

        if return_value == 0 {
            Ok(())
        } else {
            Err(VerificationError::from_error_code(return_value).into())
        }
    }

    fn ed25519_batch_verify(
        &self,
        prehash_msgs: &[&[u8]],
        sigs: &[&[u8]],
        pks: &[&[u8]],
    ) -> StdResult<()> {
        let prehash_msgs = encode_sections(prehash_msgs)?;
        let prehash_msgs_region = Region::build(&prehash_msgs);
        let prehash_msgs_ptr = &*prehash_msgs_region as *const Region;

        let sigs = encode_sections(sigs)?;
        let sigs_region = Region::build(&sigs);
        let sigs_ptr = &*sigs_region as *const Region;

        let pks = encode_sections(pks)?;
        let pks_region = Region::build(&pks);
        let pks_ptr = &*pks_region as *const Region;

        let return_value = unsafe {
            ed25519_batch_verify(
                prehash_msgs_ptr as usize,
                sigs_ptr as usize,
                pks_ptr as usize,
            )
        };

        if return_value == 0 {
            Ok(())
        } else {
            Err(VerificationError::from_error_code(return_value).into())
        }
    }
}

// ---------------------------------- querier ----------------------------------

/// A zero-size wrapper over the `query_chain` FFI function.
pub struct ExternalQuerier;

impl Querier for ExternalQuerier {
    fn query_chain(&self, req: Query) -> StdResult<QueryResponse> {
        let req_bytes = req.to_borsh_vec()?;
        let req_region = Region::build(&req_bytes);
        let req_ptr = &*req_region as *const Region;

        let res_ptr = unsafe { query_chain(req_ptr as usize) };
        let res_bytes = unsafe { Region::consume(res_ptr as *mut Region) };
        let res: GenericResult<QueryResponse> = res_bytes.deserialize_borsh()?;

        res.map_err(StdError::host)
    }
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
