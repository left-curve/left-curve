use {
    crate::utils::{Nodes, launch_nodes, setup_tracing, temp_dir},
    dango_testing::{Preset, TestOption},
    dango_types::constants::usdc,
    grug::{Coins, Message, NonEmpty, ResultExt, Signer},
    grug_db_disk_lite::DiskDbLite,
    grug_db_memory::MemDb,
    grug_malachite::{RawTx, mempool::MempoolMsg},
    std::time::Duration,
};

pub mod utils;

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multiple() {
    setup_tracing();

    let Nodes {
        actors,
        mut accounts,
        ..
    } = launch_nodes([
        (tracing::span!(tracing::Level::INFO, "node-1"), MemDb::new()),
        (tracing::span!(tracing::Level::INFO, "node-2"), MemDb::new()),
        (tracing::span!(tracing::Level::INFO, "node-3"), MemDb::new()),
    ])
    .await;

    tokio::time::sleep(Duration::from_secs(15)).await;

    // try broadcast a failing tx
    actors[1]
        .mempool
        .call(
            |reply| MempoolMsg::Add {
                tx: RawTx::from_bytes(vec![]),
                reply,
            },
            None,
        )
        .await
        .unwrap()
        .unwrap()
        .should_fail();

    tokio::time::sleep(Duration::from_secs(8)).await;

    // broadcast a tx
    let msg = Message::transfer(
        accounts[1].user2.address.into_inner(),
        Coins::one(usdc::DENOM.clone(), 100).unwrap(),
    )
    .unwrap();

    let tx = accounts
        .get_mut(1)
        .unwrap()
        .user1
        .sign_transaction(
            NonEmpty::new_unchecked(vec![msg]),
            &TestOption::preset_test().chain_id,
            1_000_000,
        )
        .unwrap();

    actors[1]
        .mempool
        .call(
            |reply| MempoolMsg::Add {
                tx: RawTx::from_tx(tx).unwrap(),
                reply,
            },
            None,
        )
        .await
        .unwrap()
        .unwrap()
        .should_succeed()
        .result
        .should_succeed();

    tokio::signal::ctrl_c().await.unwrap();
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disk_db() {
    setup_tracing();

    let mut nodes = launch_nodes([
        (
            tracing::span!(tracing::Level::INFO, "node-1"),
            DiskDbLite::open(temp_dir("node-1")).unwrap(),
        ),
        (
            tracing::span!(tracing::Level::INFO, "node-2"),
            DiskDbLite::open(temp_dir("node-2")).unwrap(),
        ),
        (
            tracing::span!(tracing::Level::INFO, "node-3"),
            DiskDbLite::open(temp_dir("node-3")).unwrap(),
        ),
        (
            tracing::span!(tracing::Level::INFO, "node-4"),
            DiskDbLite::open(temp_dir("node-4")).unwrap(),
        ),
    ])
    .await;

    tokio::time::sleep(Duration::from_secs(15)).await;

    tracing::warn!("killing node-1");
    nodes.stop_actor(0);

    tokio::time::sleep(Duration::from_secs(15)).await;

    tracing::warn!("relaunching node-1");
    nodes.relaunch_actor(0).await;

    tokio::time::sleep(Duration::from_secs(20)).await;
}
