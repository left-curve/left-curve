use metrics::describe_counter;

pub fn init_indexer_metrics() {
    describe_counter!("indexer.blocks.total", "Total indexed blocks");
    describe_counter!("indexer.transactions.total", "Total indexed transactions");
    describe_counter!("indexer.messages.total", "Total indexed messages");
    describe_counter!("indexer.events.total", "Total indexed events");
}
