use metrics::{describe_counter, describe_histogram};

pub fn init_indexer_metrics() {
    describe_counter!("indexer.blocks.total", "Total indexed blocks");
    describe_counter!("indexer.transactions.total", "Total indexed transactions");
    describe_counter!("indexer.messages.total", "Total indexed messages");
    describe_counter!("indexer.events.total", "Total indexed events");
    describe_counter!("indexer.blocks.processed.total", "Total blocks processed");
    describe_counter!(
        "indexer.blocks.deleted.total",
        "Total blocks deleted from disk"
    );
    describe_counter!("indexer.blocks.compressed.total", "Total blocks compressed");
    describe_counter!(
        "indexer.pubsub.published.total",
        "Total pubsub messages published"
    );
    describe_counter!(
        "indexer.previous_blocks.processed.total",
        "Total previous unindexed blocks processed"
    );
    describe_counter!("indexer.errors.save.total", "Total database save errors");
    describe_counter!("indexer.errors.pubsub.total", "Total pubsub errors");
    describe_counter!(
        "indexer.database.errors.total",
        "Total database operation errors"
    );
    describe_histogram!(
        "indexer.block_save.duration",
        "Block save duration in seconds"
    );
}
