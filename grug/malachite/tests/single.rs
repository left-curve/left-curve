use grug_db_memory::MemDb;

use crate::utils::{launch_nodes, setup_tracing};

pub mod utils;

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn single() {
    setup_tracing();

    launch_nodes([(tracing::span!(tracing::Level::INFO, "node-1"), MemDb::new())]).await;

    tokio::signal::ctrl_c().await.unwrap();
}
