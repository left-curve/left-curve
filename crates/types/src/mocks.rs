use {
    crate::{Addr, Api, Order, Record, StdError, StdResult, Storage},
    grug_crypto::{secp256k1_verify, secp256r1_verify},
    std::{collections::BTreeMap, iter, ops::Bound},
};

// ---------------------------------- storage ----------------------------------

/// An in-memory KV store for testing purpose.
#[derive(Default, Debug, Clone)]
pub struct MockStorage {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MockStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        // BTreeMap::range panics if
        // 1. start > end, or
        // 2. start == end and both are exclusive
        // for us, since we interpret min as inclusive and max as exclusive,
        // only the 1st case apply. however, we don't want to panic, we just
        // return an empty iterator.
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return Box::new(iter::empty());
            }
        }

        let min = min.map_or(Bound::Unbounded, |bytes| Bound::Included(bytes.to_vec()));
        let max = max.map_or(Bound::Unbounded, |bytes| Bound::Excluded(bytes.to_vec()));
        let iter = self.data.range((min, max)).map(|(k, v)| (k.clone(), v.clone()));

        if order == Order::Ascending {
            Box::new(iter)
        } else {
            Box::new(iter.rev())
        }
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.data.insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.data.remove(key);
    }
}

// ------------------------------------ api ------------------------------------

pub struct MockApi;

impl Api for MockApi {
    fn debug(&self, addr: &Addr, msg: &str) {
        println!("Contract emitted debug message! addr = {addr}, msg = {msg}");
    }

    fn secp256k1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        secp256k1_verify(msg_hash, sig, pk).map_err(|_| StdError::VerificationFailed)
    }

    fn secp256r1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        secp256r1_verify(msg_hash, sig, pk).map_err(|_| StdError::VerificationFailed)
    }
}
