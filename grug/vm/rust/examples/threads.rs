//! This example showcases multi-threading in the Rust VM.
//!
//! When instantiating the contract, we load some random data into the contract
//! storage. The random data is a set of key-value pairs:
//!
//! ```plain
//! (prefix, key) => value
//! ```
//!
//! For example, prefix is `"alice"`, key is 123, value is 456.
//!
//! The contract's job is to sum up the values corresponding to all keys, for
//! each prefix. It will do this either using a single thread, one prefix at a
//! time, or using multiple threads, one thread per prefix.
//!
//! This example mimics order clearing in our order book contract, where the
//! prefix is the trading pair ({base_denom, quote_denom}) and the KV pairs are
//! the orders. We want to clear orders in multiple trading pairs in parallel.

mod example {
    use {
        grug_storage::{Item, Map},
        grug_types::{MutableCtx, Order, Response, StdResult, Storage},
        std::{collections::BTreeMap, thread},
    };

    pub const PREFIXES: [&str; 5] = ["alice", "bob", "charlie", "dave", "eve"];

    pub const DATA: Map<(&str, u64), u64> = Map::new("data");

    pub const MULTI_THREAD_RESULT: Item<BTreeMap<String, u64>> = Item::new("multi_thread_result");

    pub const SINGLE_THREAD_RESULT: Item<BTreeMap<String, u64>> = Item::new("single_thread_result");

    pub struct InstantiateMsg {
        pub data: BTreeMap<&'static str, BTreeMap<u64, u64>>,
    }

    pub enum ExecuteMsg {
        ComputeMultiThread {},
        ComputeSingleThread {},
    }

    pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
        for (prefix, keys) in msg.data {
            for (key, value) in keys {
                DATA.save(ctx.storage, (prefix, key), &value)?;
            }
        }

        Ok(Response::new())
    }

    pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
        match msg {
            ExecuteMsg::ComputeMultiThread {} => compute_multi_thread(ctx),
            ExecuteMsg::ComputeSingleThread {} => compute_single_thread(ctx),
        }
    }

    fn compute_single_thread(ctx: MutableCtx) -> StdResult<Response> {
        let sums = PREFIXES
            .into_iter()
            .map(|prefix| {
                DATA.prefix(prefix)
                    .values(ctx.storage, None, None, Order::Ascending)
                    .try_fold(0, |sum, x| x.map(|x| sum + x))
                    .map(|sum| (prefix.to_string(), sum))
            })
            .collect::<StdResult<BTreeMap<_, _>>>()?;

        SINGLE_THREAD_RESULT.save(ctx.storage, &sums)?;

        Ok(Response::new())
    }

    fn compute_multi_thread(ctx: MutableCtx) -> StdResult<Response> {
        // Note: must use a _scoped_ thread to ensure the thread doesn't outlive
        // `ctx.storage`.
        let sums = thread::scope(|s| {
            PREFIXES
                .into_iter()
                .map(|prefix| {
                    // Cast the storage to an _immutable_ reference so that it
                    // can be shared among threads.
                    let storage = ctx.storage as &dyn Storage;
                    s.spawn(move || {
                        DATA.prefix(prefix)
                            .values(storage, None, None, Order::Ascending)
                            .try_fold(0, |sum, x| x.map(|x| sum + x))
                            .map(|sum| (prefix.to_string(), sum))
                    })
                    .join()
                    .unwrap_or_else(|err| panic!("thread panicked: {err:?}"))
                })
                .collect::<StdResult<BTreeMap<_, _>>>()
        })?;

        MULTI_THREAD_RESULT.save(ctx.storage, &sums)?;

        Ok(Response::new())
    }
}

fn main() {}
