use {
    crate::Region,
    cw_types::{
        from_json_slice, to_json_vec, Api, GenericResult, Order, Querier, QueryRequest,
        QueryResponse, Record, StdError, StdResult, Storage,
    },
};

// these are the method that the host must implement.
// we use usize to denote memory addresses, and i32 to denote other data.
extern "C" {
    // database operations.
    //
    // note that these methods are not fallible. the reason is that if a DB op
    // indeed fails, the host should have thrown an error and kill the execution.
    // from the Wasm module's perspective, if a response is received, the DB op
    // must have succeeded.
    //
    // read ops (no state mutation):
    fn db_read(key_ptr: usize) -> usize;
    fn db_scan(min_ptr: usize, max_ptr: usize, order: i32) -> i32;
    fn db_next(iterator_id: i32) -> usize;

    // write ops (mutate the state):
    fn db_write(key_ptr: usize, value_ptr: usize);
    fn db_remove(key_ptr: usize);

    // print a debug message to the client's CLI output.
    fn debug(addr_ptr: usize, msg_ptr: usize);

    // send a query request to the chain.
    // not to be confused with the query export.
    fn query_chain(req: usize) -> usize;

    // crypto methods
    // return value of 0 means ok; any value other than 0 means error.
    fn secp256k1_verify(msg_hash_ptr: usize, sig_ptr: usize, pk_ptr: usize) -> i32;
    fn secp256r1_verify(msg_hash_ptr: usize, sig_ptr: usize, pk_ptr: usize) -> i32;
}

// ---------------------------------- storage ----------------------------------

/// A zero-size convenience wrapper around the database imports. Provides more
/// ergonomic functions.
///
/// For entry points where state mutation is allowed (such as instantiate and
/// execute) a mutable reference of ExternalStorage is included in the context.
/// For entry points where state mutation isn't allowed (such as query), an
/// immutable reference is included. This prevents the contract from calling
/// the write/remove methods. Of course, the host must also set safeguards!
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
        // NOTE: key_ptr goes out of scope here, so the Region is dropped.
        // however, `key` is NOT dropped, since we're only working with a
        // borrowed reference here.
        // same case with `write` and `remove`.
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        // IMPORTANT: we must to keep the Regions in scope until end of the func
        // make sure to se `as_ref` so that the Regions don't get consumed
        let min_region = min.map(Region::build);
        let min_ptr = get_optional_region_ptr(min_region.as_ref());

        let max_region = max.map(Region::build);
        let max_ptr = get_optional_region_ptr(max_region.as_ref());

        let iterator_id = unsafe { db_scan(min_ptr, max_ptr, order.into()) };

        Box::new(ExternalIterator {
            iterator_id,
        })
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
}

pub struct ExternalIterator {
    iterator_id: i32,
}

impl Iterator for ExternalIterator {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        let ptr = unsafe { db_next(self.iterator_id) };

        // the host returning a zero pointer means iteration has finished
        if ptr == 0 {
            return None;
        }

        unsafe { Some(split_tail(Region::consume(ptr as *mut Region))) }
    }
}

// clippy has a false positive here. we have to take Option<&Box<Region>>,
// not Option<&Region>
#[allow(clippy::borrowed_box)]
fn get_optional_region_ptr(maybe_region: Option<&Box<Region>>) -> usize {
    // a zero memory address tells the host that no data has been loaded into
    // memory. in case of db_scan, it means the bound is None.
    let Some(region) = maybe_region else {
        return 0;
    };

    (region.as_ref() as *const Region) as usize
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
#[inline]
fn split_tail(mut data: Vec<u8>) -> Record {
    // pop two bytes from the end, must both be Some
    let (Some(byte1), Some(byte2)) = (data.pop(), data.pop()) else {
        panic!("[ExternalIterator]: can't read length suffix");
    };

    // note the order here between the two bytes
    let key_len = u16::from_be_bytes([byte2, byte1]);
    let value = data.split_off(key_len.into());

    (data, value)
}

// ------------------------------------ api ------------------------------------

pub struct ExternalApi;

impl Api for ExternalApi {
    fn debug(&self, addr: &[u8], msg: &[u8]) {
        let addr_region = Region::build(addr);
        let addr_ptr = &*addr_region as *const Region;
        let msg_region = Region::build(msg);
        let msg_ptr = &*msg_region as *const Region;

        unsafe { debug(addr_ptr as usize, msg_ptr as usize) }
    }

    fn secp256k1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        let msg_hash_region = Region::build(msg_hash.as_ref());
        let msg_hash_ptr = &*msg_hash_region as *const Region;

        let sig_region = Region::build(sig.as_ref());
        let sig_ptr = &*sig_region as *const Region;

        let pk_region = Region::build(pk.as_ref());
        let pk_ptr = &*pk_region as *const Region;

        let return_value = unsafe {
            secp256k1_verify(msg_hash_ptr as usize, sig_ptr as usize, pk_ptr as usize)
        };

        if return_value == 0 {
            Ok(())
        } else {
            // TODO: more useful error codes
            Err(StdError::VerificationFailed)
        }
    }

    fn secp256r1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        let msg_hash_region = Region::build(msg_hash.as_ref());
        let msg_hash_ptr = &*msg_hash_region as *const Region;

        let sig_region = Region::build(sig.as_ref());
        let sig_ptr = &*sig_region as *const Region;

        let pk_region = Region::build(pk.as_ref());
        let pk_ptr = &*pk_region as *const Region;

        let return_value = unsafe {
            secp256r1_verify(msg_hash_ptr as usize, sig_ptr as usize, pk_ptr as usize)
        };

        if return_value == 0 {
            Ok(())
        } else {
            // TODO: more useful error codes
            Err(StdError::VerificationFailed)
        }
    }
}

// ---------------------------------- querier ----------------------------------

pub struct ExternalQuerier;

impl Querier for ExternalQuerier {
    fn query_chain(&self, req: &QueryRequest) -> StdResult<QueryResponse> {
        let req_bytes = to_json_vec(req)?;
        let req_region = Region::build(&req_bytes);
        let req_ptr = &*req_region as *const Region;

        let res_ptr = unsafe { query_chain(req_ptr as usize) };
        let res_bytes = unsafe { Region::consume(res_ptr as *mut Region) };
        let res: GenericResult<QueryResponse> = from_json_slice(&res_bytes)?;

        res.into_std_result()
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
