//! Production-path backfill → live → full query coverage.
//!
//! Drive the mock node to 100 blocks BEFORE starting the indexer, so the
//! `RemoteBlockSource` connects mid-history exactly like a production cold
//! start. The live tail (`since=None`) streams only blocks *newer* than the
//! tip it observes, so the fetcher must backfill the whole history below through
//! the REST `/block/full/range` API. The chain must keep producing for the live
//! tail to observe a tip at all — so we pump blocks until the backfill catches
//! up (the "poi continua a farne fare" part), then exercise every GraphQL feed.
//!
//! Assertions survive two unknowns: the genesis block-height offset (counts are
//! of *broadcast* txs, not absolute heights) and gas mechanics. The test preset
//! runs with a zero gas-fee rate, so no fee transfers fire and event volumes are
//! exact — one `Transfer` + two bank contract-events (`sent`/`received`) per
//! transfer — each cross-checked against its per-name totals and printed as a
//! diagnostic. `user4` is a recipient that never sends, keeping its per-address
//! assertions gas-neutral.

use {
    dango_indexer_historical_testing::{PendingEnv, broadcast_transfer},
    dango_primitives::{Addr, FlatCategory},
    sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement},
    std::time::Duration,
};

/// Bulk transfers before the indexer starts (+1 marker = 100 pre-blocks).
const PRE_BULK: usize = 99;
const WAIT: Duration = Duration::from_secs(90);

#[tokio::test(flavor = "multi_thread")]
async fn backfill_then_live_then_query_coverage() {
    let mut pending = PendingEnv::setup().await.expect("set up mock node + db");

    // Addresses read by reference (no partial move of `pending.accounts`).
    let user1 = pending.accounts.user1.address.inner().to_string();
    let user3 = pending.accounts.user3.address.inner().to_string();
    let user4 = pending.accounts.user4.address.inner().to_string();
    let addr_user2 = *pending.accounts.user2.address.inner();
    let addr_user4 = *pending.accounts.user4.address.inner();
    let bank = pending.contracts.bank.to_string();

    // ---- produce 100 blocks BEFORE the indexer ----
    // Block 1: the MARKER, user3 -> user4. A discriminator that appears nowhere
    // else and is the lowest block — only the fetcher can reach it.
    let marker_hash = broadcast_transfer(
        &pending.client,
        &mut pending.accounts.user3,
        addr_user4,
        777,
    )
    .await
    .expect("broadcast the marker");
    // Blocks 2..=100: bulk user1 -> user2.
    let mut last_pre_hash = marker_hash;
    for _ in 0..PRE_BULK {
        last_pre_hash = broadcast_transfer(
            &pending.client,
            &mut pending.accounts.user1,
            addr_user2,
            100,
        )
        .await
        .expect("broadcast a bulk transfer");
    }

    // ---- start the indexer; it connects at the tip and backfills below ----
    let mut env = pending.start_indexer().await.expect("start the indexer");

    // Keep producing — the live tail (`since=None`) only advances on NEW blocks,
    // and the healer backfills everything below the tip the live tail observes.
    // Pump user1 -> user2 blocks until the last pre-indexer block is caught up:
    // that means the fetcher backfilled the whole history below the tip (none of
    // the 100 pre-blocks could have arrived live — they predate the
    // subscription). Track the live count so the assertions stay exact.
    let last_pre = last_pre_hash.to_string();
    let deadline = std::time::Instant::now() + WAIT;
    let mut live_hashes = Vec::new();
    loop {
        live_hashes.push(
            broadcast_transfer(&env.client, &mut env.accounts.user1, addr_user2, 100)
                .await
                .expect("broadcast a live transfer"),
        );
        if env.is_tx_indexed(&last_pre).await.expect("index check") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "backfill did not catch up within {WAIT:?}",
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let live_count = live_hashes.len();
    let total = 100 + live_count;

    // The live tail caught up to the latest block it produced — every live block
    // above the backfilled history is indexed too.
    let last_live = live_hashes.last().unwrap().to_string();
    env.wait_for_tx_indexed(&last_live, WAIT)
        .await
        .expect("live tail to reach the latest block");

    // The marker (block 1) is indexed — concrete proof the fetcher reached the
    // very bottom of the history (the live tail never saw blocks <= the tip).
    let marker = env
        .wait_for_tx_indexed(&marker_hash.to_string(), WAIT)
        .await
        .expect("fetcher to reach the marker (bottom) block");
    let marker_node = &marker["transactionsByHash"][0];
    let marker_height = marker_node["blockHeight"].as_u64().expect("marker height");
    assert_eq!(marker_node["sender"].as_str().unwrap(), user3);
    assert!(marker_node["success"].as_bool().unwrap());

    // Precondition for the exact `count(*)` below: the window must stay below
    // the first perps cronjob. The preset schedules it every minute and blocks
    // are 250 ms apart, so the first cron unit lands at ~block 240; from there
    // every block adds a `kind=Cron` row that would inflate the count. The live
    // pump is deadline-bounded, not block-bounded, so assert zero cron units
    // explicitly — a degraded run that pumps past block 240 then fails here with
    // a clear cause instead of a confusing off-by-N count mismatch.
    assert_eq!(
        count(
            env.conn(),
            &format!(
                "SELECT count(*) AS n FROM activity_transactions WHERE kind = {}",
                FlatCategory::Cron as i16
            ),
        )
        .await,
        0,
        "no cron units expected below ~block 240 (saw {total} blocks)",
    );

    // Complete, contiguous catch-up: one tx per block, no gaps, no crons, and
    // the marker sits at the very bottom of the range.
    assert_eq!(
        count(
            env.conn(),
            "SELECT count(*) AS n FROM activity_transactions"
        )
        .await,
        total as i64,
        "every broadcast block is indexed exactly once",
    );
    assert_eq!(
        count(
            env.conn(),
            "SELECT count(DISTINCT block_height) AS n FROM activity_transactions",
        )
        .await,
        total as i64,
        "exactly one tx per block",
    );
    assert_eq!(
        count(
            env.conn(),
            "SELECT max(block_height) - min(block_height) + 1 AS n FROM activity_transactions",
        )
        .await,
        total as i64,
        "the indexed block range is contiguous (no gaps)",
    );
    assert_eq!(
        marker_height,
        count(
            env.conn(),
            "SELECT min(block_height) AS n FROM activity_transactions"
        )
        .await as u64,
        "the marker is the lowest indexed block",
    );
    eprintln!("pumped {live_count} live blocks; {total} blocks indexed total");

    // ===== query coverage =====

    // ---- transactionsInvolving ----
    // user1 sent every bulk + live transfer, and nothing else.
    let sent = env
        .collect_heights(
            "transactionsInvolving",
            &format!("address: \"{user1}\", role: SENDER"),
        )
        .await
        .unwrap();
    assert_eq!(
        sent.len(),
        PRE_BULK + live_count,
        "user1 sent every non-marker transfer"
    );
    assert!(
        strictly_descending(&sent),
        "transaction feeds are newest-first"
    );

    // user3 sent only the marker.
    let s3 = env
        .collect_heights(
            "transactionsInvolving",
            &format!("address: \"{user3}\", role: SENDER"),
        )
        .await
        .unwrap();
    assert_eq!(s3, vec![marker_height], "user3 sent only the marker");

    // user4 sent nothing.
    let s4 = env
        .collect_heights(
            "transactionsInvolving",
            &format!("address: \"{user4}\", role: SENDER"),
        )
        .await
        .unwrap();
    assert!(s4.is_empty(), "user4 never sent a tx");

    // user4 participates only in the marker (as recipient).
    let p4 = env
        .collect_heights(
            "transactionsInvolving",
            &format!("address: \"{user4}\", role: PARTICIPANT"),
        )
        .await
        .unwrap();
    assert_eq!(
        p4,
        vec![marker_height],
        "user4 participates only in the marker"
    );

    // kind: CRON → none in this window (the exclusion case).
    let crons = env
        .collect_heights(
            "transactionsInvolving",
            &format!("address: \"{user1}\", kind: CRON"),
        )
        .await
        .unwrap();
    assert!(crons.is_empty(), "no cron units in this window");

    // kind: TRANSACTION → all of user1's units.
    let txs = env
        .collect_heights(
            "transactionsInvolving",
            &format!("address: \"{user1}\", kind: TRANSACTION"),
        )
        .await
        .unwrap();
    assert_eq!(txs.len(), PRE_BULK + live_count);

    // explicit first-page pagination shape.
    let page = env
        .query(&format!(
            "{{ transactionsInvolving(address: \"{user1}\", role: SENDER, first: 50) \
             {{ edges {{ node {{ blockHeight }} }} pageInfo {{ hasNextPage }} }} }}"
        ))
        .await
        .unwrap();
    assert_eq!(
        page["transactionsInvolving"]["edges"]
            .as_array()
            .unwrap()
            .len(),
        50,
        "a page is capped at 50",
    );
    assert!(
        page["transactionsInvolving"]["pageInfo"]["hasNextPage"]
            .as_bool()
            .unwrap()
    );

    // ---- transactionsByHash ----
    let by_hash = env
        .query(&format!(
            "{{ transactionsByHash(hash: \"{marker_hash}\") {{ blockHeight sender }} }}"
        ))
        .await
        .unwrap();
    assert_eq!(by_hash["transactionsByHash"].as_array().unwrap().len(), 1);
    // an unknown hash maps to nothing.
    let unknown_hash = "0".repeat(64);
    let none = env
        .query(&format!(
            "{{ transactionsByHash(hash: \"{unknown_hash}\") {{ blockHeight }} }}"
        ))
        .await
        .unwrap();
    assert!(none["transactionsByHash"].as_array().unwrap().is_empty());

    // ---- eventsByType ----
    let transfers = env
        .collect_heights("eventsByType", "type: TRANSFER")
        .await
        .unwrap();
    let contract_events = env
        .collect_heights("eventsByType", "type: CONTRACT_EVENT")
        .await
        .unwrap();
    eprintln!(
        "eventsByType: TRANSFER={} CONTRACT_EVENT={}",
        transfers.len(),
        contract_events.len()
    );
    assert_eq!(
        transfers.len(),
        total,
        "exactly one Transfer event per transfer"
    );
    assert!(non_increasing(&transfers), "event feeds are newest-first");
    assert_eq!(
        contract_events.len(),
        2 * total,
        "exactly the bank `sent` + `received` per transfer",
    );
    assert!(non_increasing(&contract_events));

    // ---- eventsInvolving ----
    // user4 is a party to the marker's Transfer + bank `sent` + bank `received`.
    let inv4 = env
        .collect_heights("eventsInvolving", &format!("address: \"{user4}\""))
        .await
        .unwrap();
    assert_eq!(
        inv4,
        vec![marker_height; 3],
        "user4 is a party to exactly the marker's three events",
    );
    let inv4_transfer = env
        .collect_heights(
            "eventsInvolving",
            &format!("address: \"{user4}\", type: TRANSFER"),
        )
        .await
        .unwrap();
    assert_eq!(inv4_transfer, vec![marker_height]);
    let inv4_contract = env
        .collect_heights(
            "eventsInvolving",
            &format!("address: \"{user4}\", type: CONTRACT_EVENT"),
        )
        .await
        .unwrap();
    assert_eq!(
        inv4_contract,
        vec![marker_height; 2],
        "the marker's `sent` + `received`"
    );

    // ---- contractEvents (bank) ----
    let sent_events = env
        .collect_heights(
            "contractEvents",
            &format!("contract: \"{bank}\", names: [\"sent\"]"),
        )
        .await
        .unwrap();
    let received_events = env
        .collect_heights(
            "contractEvents",
            &format!("contract: \"{bank}\", names: [\"received\"]"),
        )
        .await
        .unwrap();
    eprintln!(
        "bank contractEvents: sent={} received={}",
        sent_events.len(),
        received_events.len()
    );
    assert_eq!(
        sent_events.len(),
        total,
        "exactly one bank `sent` per transfer"
    );
    assert_eq!(
        received_events.len(),
        total,
        "exactly one bank `received` per transfer"
    );
    // an unknown contract has no events.
    let unknown_contract = Addr::try_from(vec![0xCDu8; 20]).unwrap().to_string();
    let no_events = env
        .collect_heights(
            "contractEvents",
            &format!("contract: \"{unknown_contract}\""),
        )
        .await
        .unwrap();
    assert!(no_events.is_empty(), "an unknown contract has no events");

    // ---- contractEventsInvolving ----
    let ci4 = env
        .collect_heights(
            "contractEventsInvolving",
            &format!("address: \"{user4}\", contract: \"{bank}\""),
        )
        .await
        .unwrap();
    assert_eq!(
        ci4,
        vec![marker_height; 2],
        "user4's bank sent + received in the marker"
    );
    let ci4_received = env
        .collect_heights(
            "contractEventsInvolving",
            &format!("address: \"{user4}\", contract: \"{bank}\", names: [\"received\"]"),
        )
        .await
        .unwrap();
    assert_eq!(ci4_received, vec![marker_height]);
    let ci4_sent = env
        .collect_heights(
            "contractEventsInvolving",
            &format!("address: \"{user4}\", contract: \"{bank}\", names: [\"sent\"]"),
        )
        .await
        .unwrap();
    assert_eq!(ci4_sent, vec![marker_height]);

    // ===== on-demand payload hydration — the read paths back to the store =====

    // block(height): the full `{ block, outcome }` read straight from the
    // RocksDB store the fetcher populated, for the marker's own height.
    let block = env
        .query(&format!("{{ block(height: {marker_height}) }}"))
        .await
        .unwrap();
    assert_eq!(
        block["block"]["block"]["info"]["height"].as_u64(),
        Some(marker_height),
        "block(height) returns the stored block at that height",
    );
    assert!(
        block["block"]["outcome"].is_object(),
        "...with its execution outcome"
    );
    // a height the source does not hold → null, no error.
    let absent = env
        .query(&format!("{{ block(height: {}) }}", (total + 1000) as u64))
        .await
        .unwrap();
    assert!(absent["block"].is_null(), "an un-ingested height is null");

    // Transaction.tx / Transaction.outcome: hydrated from the unit's block via
    // the shared BlockLoader (→ source.get → RocksDB).
    let hydrated = env
        .query(&format!(
            "{{ transactionsByHash(hash: \"{marker_hash}\") {{ tx outcome }} }}"
        ))
        .await
        .unwrap();
    let unit = &hydrated["transactionsByHash"][0];
    assert!(
        unit["tx"].is_object(),
        "tx is hydrated from the block store"
    );
    assert!(
        unit["outcome"]["transaction"].is_object(),
        "outcome is hydrated from the block store",
    );

    // Event.data: the priority Transfer payload, served from the inline
    // event_data join (decompressed) without a block load.
    let with_data = env
        .query(&format!(
            "{{ eventsInvolving(address: \"{user4}\", type: TRANSFER, first: 1) \
             {{ edges {{ node {{ data }} }} }} }}"
        ))
        .await
        .unwrap();
    let payload = &with_data["eventsInvolving"]["edges"][0]["node"]["data"];
    assert!(
        !payload.is_null(),
        "the transfer event payload round-trips from event_data"
    );
}

/// Run a SQL query that returns a single `bigint` column aliased `n`.
async fn count(conn: &DatabaseConnection, sql: &str) -> i64 {
    let row = conn
        .query_one(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await
        .expect("query")
        .expect("one row");
    row.try_get::<i64>("", "n").expect("column n")
}

fn strictly_descending(v: &[u64]) -> bool {
    v.windows(2).all(|w| w[0] > w[1])
}

fn non_increasing(v: &[u64]) -> bool {
    v.windows(2).all(|w| w[0] >= w[1])
}
