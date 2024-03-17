use {
    crate::{
        from_json_value, to_json_value, AccountResponse, Addr, Batch, Binary, Coins, Hash,
        InfoResponse, Op, Order, QueryRequest, QueryResponse, Record, StdResult, Uint128,
    },
    dyn_clone::DynClone,
    serde::{de::DeserializeOwned, ser::Serialize},
};

// ---------------------------------- storage ----------------------------------

/// Describing a KV store that supports read, write, and iteration.
///
/// Note that the store must be clone-able, which is required by Wasmer runtime.
/// We can't use the std library Clone trait, which is not object-safe.
/// We use DynClone (https://crates.io/crates/dyn-clone) instead, which is
/// object-safe, and use the `clone_trait_object!` macro below to derive std
/// Clone trait for any type that implements Storage.
pub trait Storage: DynClone {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Iterate over data in the KV store under the given bounds and order.
    /// Minimum bound is inclusive, maximum bound is exclusive.
    /// If min > max, an empty iterator is to be returned.
    ///
    /// NOTE: Rust's BTreeMap panics if max > max. We don't want this behavior.
    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);

    /// Perform a batch of writes and removes altogether, ideally atomically.
    ///
    /// The batch is provided by value instead of by reference (unlike other
    /// trait methods above) because in some implementations a copy/clone can be
    /// avoided this way, improving performance.
    ///
    /// The default implementation here is just looping through the ops and
    /// applying them one by one, which is inefficient and not atomic.
    /// Overwrite this implementation if there are more efficient approaches.
    fn flush(&mut self, batch: Batch) {
        for (key, op) in batch {
            if let Op::Insert(value) = op {
                self.write(&key, &value);
            } else {
                self.remove(&key);
            }
        }
    }
}

// derive std Clone trait for any type that implements Storage
dyn_clone::clone_trait_object!(Storage);

// ------------------------------------ api ------------------------------------

// note: I prefer to use generics (e.g. `impl AsRef<[u8]>`) instead of `&[u8]`
// for the method parameters, but by doing that the trait won't be object-safe
// (i.e. we won't be able to do `&dyn Api`). traits with methods that have
// generic methods can't be object-safe.
pub trait Api {
    /// Verify an Secp256k1 signature with the given hashed message and public
    /// key.
    ///
    /// Note: this function takes the hash of the message, not the prehash.
    fn secp256k1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()>;

    /// Verify an Secp256r1 signature with the given hashed message and public
    /// key.
    ///
    /// Note: this function takes the hash of the message, not the prehash.
    fn secp256r1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()>;
}

// ---------------------------------- querier ----------------------------------

pub trait Querier {
    /// Make a query. This is the only method that the context needs to manually
    /// implement. The other methods will be implemented automatically.
    fn query(&self, req: &QueryRequest) -> StdResult<QueryResponse>;

    fn query_info(&self) -> StdResult<InfoResponse> {
        self.query(&QueryRequest::Info {}).map(|res| res.as_info())
    }

    fn query_balance(&self, address: Addr, denom: String) -> StdResult<Uint128> {
        self.query(&QueryRequest::Balance {
            address,
            denom,
        })
        .map(|res| res.as_balance().amount)
    }

    fn query_balances(
        &self,
        address: Addr,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Coins> {
        self.query(&QueryRequest::Balances {
            address,
            start_after,
            limit,
        })
        .map(|res| res.as_balances())
    }

    fn query_supply(&self, denom: String) -> StdResult<Uint128> {
        self.query(&QueryRequest::Supply {
            denom,
        })
        .map(|res| res.as_supply().amount)
    }

    fn query_supplies(&self, start_after: Option<String>, limit: Option<u32>) -> StdResult<Coins> {
        self.query(&QueryRequest::Supplies {
            start_after,
            limit,
        })
        .map(|res| res.as_supplies())
    }

    fn query_code(&self, hash: Hash) -> StdResult<Binary> {
        self.query(&QueryRequest::Code {
            hash,
        })
        .map(|res| res.as_code())
    }

    fn query_codes(&self, start_after: Option<Hash>, limit: Option<u32>) -> StdResult<Vec<Hash>> {
        self.query(&QueryRequest::Codes {
            start_after,
            limit,
        })
        .map(|res| res.as_codes())
    }

    fn query_account(&self, address: Addr) -> StdResult<AccountResponse> {
        self.query(&QueryRequest::Account {
            address,
        })
        .map(|res| res.as_account())
    }

    fn query_accounts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
    ) -> StdResult<Vec<AccountResponse>> {
        self.query(&QueryRequest::Accounts {
            start_after,
            limit,
        })
        .map(|res| res.as_accounts())
    }

    fn query_wasm_raw(&self, contract: Addr, key: Binary) -> StdResult<Option<Binary>> {
        self.query(&QueryRequest::WasmRaw {
            contract,
            key,
        })
        .map(|res| res.as_wasm_raw().value)
    }

    fn query_wasm_smart<M: Serialize, R: DeserializeOwned>(
        &self,
        contract: Addr,
        msg: &M,
    ) -> StdResult<R> {
        self.query(&QueryRequest::WasmSmart {
            contract,
            msg: to_json_value(msg)?,
        })
        .and_then(|res| from_json_value(res.as_wasm_smart().data))
    }
}
