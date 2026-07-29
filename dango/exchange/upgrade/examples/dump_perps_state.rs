//! Dump a live chain's perps state into a fixture for the wind-down tests.
//!
//! ```sh
//! cargo run -p dango-upgrade --example dump_perps_state -- mainnet
//! cargo run -p dango-upgrade --example dump_perps_state -- testnet
//! ```
//!
//! Writes `testdata/<chain>_snapshot.json`, which `src/perps.rs`'s `#[ignore]`
//! tests load. `testdata/` is gitignored — the dump is large and goes stale as
//! the chain advances, so it is never committed.
//!
//! Everything is fetched in a single `multi` query, so the whole snapshot comes
//! from one block. That matters: paginating would straddle blocks and produce
//! an internally inconsistent snapshot, and the node only ever serves the
//! latest finalized block, so a dump cannot be pinned to a chosen height.
//!
//! Only the storage the wind-down actually touches is scanned. An unbounded
//! scan of the whole contract also works on mainnet, but it drags in the
//! referral and per-user volume history — an order of magnitude more entries
//! than the rest combined — and on testnet that exceeds the node's query gas
//! limit outright.

use {
    serde_json::{Map, Value, json},
    std::{env, fs, path::PathBuf},
};

const MAINNET_URL: &str = "https://api-mainnet.dango.zone";
const MAINNET_PERPS: &str = "0x90bc84df68d1aa59a857e04ed529e9a26edbea4f";

const TESTNET_URL: &str = "https://api-testnet.dango.zone";
const TESTNET_PERPS: &str = "0xf6344c5e2792e8f9202c58a2d88fbbde4cd3142f";

const USDC_DENOM: &str = "bridge/usdc";

/// `wasm_scan` defaults to a 30-entry page, so an explicit limit is required.
/// The largest namespace here holds a few thousand entries.
const SCAN_LIMIT: u64 = 1_000_000;

/// `Item`s, whose raw key is the bare name with no length prefix.
const ITEMS: &[&str] = &[
    "param",
    "state",
    "pair_ids",
    "next_order_id",
    "next_fill_id",
];

/// `Map`/`Set`/`IndexedMap` namespaces, including the secondary indexes, which
/// live in the same keyspace and must be restored for the map to be coherent.
const NAMESPACES: &[&str] = &[
    "pair_param",
    "pair_state",
    "us",
    "us__unlock",
    "us__cond",
    "long",
    "short",
    "bid",
    "bid__id",
    "bid__user",
    "bid__cid",
    "ask",
    "ask__id",
    "ask__user",
    "ask__cid",
    "depth",
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let chain = env::args().nth(1).unwrap_or_else(|| "mainnet".to_string());

    let (url, perps) = match chain.as_str() {
        "mainnet" => (MAINNET_URL, MAINNET_PERPS),
        "testnet" => (TESTNET_URL, TESTNET_PERPS),
        _ => anyhow::bail!("unknown chain `{chain}`; expected `mainnet` or `testnet`"),
    };

    println!("Fetching perps state from {url}...");

    // One request, so every sub-query is answered at the same block.
    let mut queries = Vec::new();

    for item in ITEMS {
        queries.push(json!({
            "wasm_raw": { "contract": perps, "key": encode(item.as_bytes()) },
        }));
    }

    for namespace in NAMESPACES {
        let (min, max) = namespace_bounds(namespace);

        queries.push(json!({
            "wasm_scan": {
                "contract": perps,
                "min": encode(&min),
                "max": encode(&max),
                "limit": SCAN_LIMIT,
            },
        }));
    }

    queries.push(json!({
        "balance": { "address": perps, "denom": USDC_DENOM },
    }));

    let response: Value = reqwest::Client::new()
        .post(format!("{url}/query"))
        .json(&json!({ "multi": queries }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let results = response
        .get("multi")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("malformed response: {response}"))?;

    anyhow::ensure!(
        results.len() == queries.len(),
        "expected {} sub-results, got {}",
        queries.len(),
        results.len()
    );

    let mut storage = Map::new();
    let mut cursor = 0;

    // `Item`s come back as a bare value, or null when unset.
    for item in ITEMS {
        let value = unwrap_ok(&results[cursor])?
            .get("wasm_raw")
            .ok_or_else(|| anyhow::anyhow!("missing wasm_raw for `{item}`"))?;

        if let Some(value) = value.as_str() {
            storage.insert(encode(item.as_bytes()), json!(value));
        }

        cursor += 1;
    }

    for namespace in NAMESPACES {
        let scanned = unwrap_ok(&results[cursor])?
            .get("wasm_scan")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow::anyhow!("missing wasm_scan for `{namespace}`"))?;

        anyhow::ensure!(
            scanned.len() < SCAN_LIMIT as usize,
            "namespace `{namespace}` hit the scan limit; the snapshot would be truncated"
        );

        for (key, value) in scanned {
            storage.insert(key.clone(), value.clone());
        }

        println!("  {namespace}: {} entries", scanned.len());

        cursor += 1;
    }

    let balance = unwrap_ok(&results[cursor])?
        .pointer("/balance/amount")
        .ok_or_else(|| anyhow::anyhow!("missing balance in response"))?
        .clone();

    let entry_count = storage.len();

    let snapshot = json!({
        "perps_address": perps,
        "storage": storage,
        "balance": balance,
    });

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(format!("{chain}_snapshot.json"));

    fs::create_dir_all(out_path.parent().unwrap())?;
    fs::write(&out_path, serde_json::to_vec(&snapshot)?)?;

    println!(
        "Saved {entry_count} entries and a USDC balance of {balance} to {}",
        out_path.display()
    );

    Ok(())
}

/// Half-open key range covering one storage namespace.
///
/// A `Map` key is `len(ns)` as two big-endian bytes, then `ns`, then the key
/// itself; so every entry in the namespace sorts within `[prefix, prefix+1)`.
fn namespace_bounds(namespace: &str) -> (Vec<u8>, Vec<u8>) {
    let mut min = Vec::with_capacity(namespace.len() + 2);
    min.extend_from_slice(&(namespace.len() as u16).to_be_bytes());
    min.extend_from_slice(namespace.as_bytes());

    let mut max = min.clone();

    // `max` is exclusive, so increment the last byte to take in the whole
    // prefix. Namespaces are ASCII, so this never overflows.
    *max.last_mut().unwrap() += 1;

    (min, max)
}

/// Binary is base64 on the wire.
fn encode(bytes: &[u8]) -> String {
    use std::fmt::Write;

    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::new();

    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);

        for i in 0..4 {
            if i <= chunk.len() {
                let _ = write!(
                    out,
                    "{}",
                    ALPHABET[(n >> (18 - 6 * i)) as usize & 0x3f] as char
                );
            } else {
                out.push('=');
            }
        }
    }

    out
}

fn unwrap_ok(result: &Value) -> anyhow::Result<&Value> {
    if let Some(err) = result.get("Err") {
        let message = err.get("error").unwrap_or(err).to_string();

        // Gas is metered across the whole `multi`, so a chain with a large
        // enough user set cannot be captured in one request. Paginating would
        // spread the snapshot over several blocks and so could catch a
        // position mid-close, which is exactly what a wind-down fixture must
        // not do — so fail rather than silently produce a torn snapshot.
        if message.contains("out of gas") {
            anyhow::bail!(
                "the node ran out of query gas part-way through the snapshot: {message}\n\nThis \
                 chain holds more state than a single consistent query can return. Only chains \
                 that fit in one request can be snapshotted."
            );
        }

        anyhow::bail!("query failed: {message}");
    }

    result
        .get("Ok")
        .ok_or_else(|| anyhow::anyhow!("malformed sub-result: {result}"))
}
