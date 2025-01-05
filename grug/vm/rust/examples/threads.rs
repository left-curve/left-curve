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

use {
    grug_testing::TestBuilder,
    grug_types::{btree_map, BorshDeExt, Coins, ResultExt},
    grug_vm_rust::ContractBuilder,
    rand::Rng,
    std::collections::BTreeMap,
};

mod example {
    use {
        grug_storage::{Item, Map},
        grug_types::{MutableCtx, Order, Response, StdResult, Storage},
        serde::{Deserialize, Serialize},
        std::{collections::BTreeMap, thread},
    };

    pub const PREFIXES: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];

    pub const DATA: Map<(&str, u64), u64> = Map::new("data");

    pub const MULTI_THREAD_RESULT: Item<BTreeMap<String, u64>> = Item::new("multi_thread_result");

    pub const SINGLE_THREAD_RESULT: Item<BTreeMap<String, u64>> = Item::new("single_thread_result");

    #[derive(Serialize, Deserialize)]
    pub struct InstantiateMsg {
        pub data: BTreeMap<String, BTreeMap<u64, u64>>,
    }

    #[derive(Serialize, Deserialize)]
    pub enum ExecuteMsg {
        ComputeMultiThread {},
        ComputeSingleThread {},
    }

    pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
        for (prefix, keys) in msg.data {
            for (key, value) in keys {
                DATA.save(ctx.storage, (&prefix, key), &value)?;
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
                        println!("thread started! prefix: {prefix}");
                        DATA.prefix(prefix)
                            .values(storage, None, None, Order::Ascending)
                            .try_fold(0, |sum, x| x.map(|x| sum + x))
                            .map(|sum| (prefix.to_string(), sum))
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|err| panic!("thread panicked: {err:?}"))
                })
                .collect::<StdResult<_>>()
        })?;

        MULTI_THREAD_RESULT.save(ctx.storage, &sums)?;

        Ok(Response::new())
    }
}

fn timed<F, R>(name: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    println!("starting... name: {name}");
    let start = std::time::Instant::now();
    let out = f();
    let elapsed = start.elapsed();
    println!("done! elapsed: {elapsed:?}");
    out
}

fn generate_random_data() -> BTreeMap<u64, u64> {
    let mut rng = rand::thread_rng();
    let count = rng.gen_range(50000..100000);
    (0..count).map(|k| (k, rng.gen_range(0..100))).collect()
}

fn main() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("sender", Coins::new())
        .set_owner("sender")
        .build();

    let example_code = ContractBuilder::new(Box::new(example::instantiate))
        .with_execute(Box::new(example::execute))
        .build();

    let example = timed("instantiate", || {
        suite
            .upload_and_instantiate(
                &mut accounts["sender"],
                example_code,
                &example::InstantiateMsg {
                    data: btree_map! {
                        "a".to_string() => generate_random_data(),
                        "b".to_string() => generate_random_data(),
                        "c".to_string() => generate_random_data(),
                        "d".to_string() => generate_random_data(),
                        "e".to_string() => generate_random_data(),
                        "f".to_string() => generate_random_data(),
                        "g".to_string() => generate_random_data(),
                        "h".to_string() => generate_random_data(),
                    },
                },
                "example",
                Some("example"),
                None,
                Coins::new(),
            )
            .should_succeed()
            .address
    });

    timed("compute_single_thread", || {
        suite
            .execute(
                &mut accounts["sender"],
                example,
                &example::ExecuteMsg::ComputeSingleThread {},
                Coins::new(),
            )
            .should_succeed();
    });

    timed("compute_multi_thread", || {
        suite
            .execute(
                &mut accounts["sender"],
                example,
                &example::ExecuteMsg::ComputeMultiThread {},
                Coins::new(),
            )
            .should_succeed();
    });

    let single_thread_result = suite
        .query_wasm_raw(example, example::SINGLE_THREAD_RESULT.path())
        .should_succeed()
        .unwrap()
        .deserialize_borsh::<BTreeMap<String, u64>>()
        .unwrap();
    let multi_thread_result = suite
        .query_wasm_raw(example, example::MULTI_THREAD_RESULT.path())
        .should_succeed()
        .unwrap()
        .deserialize_borsh::<BTreeMap<String, u64>>()
        .unwrap();

    dbg!(&single_thread_result);
    dbg!(&multi_thread_result);

    assert_eq!(single_thread_result, multi_thread_result);
}
