use {
    grug_metrics::{describe_counter, describe_histogram},
    std::sync::Once,
};

pub const LABEL_TX_PER_BLOCK: &str = "grug.app.tx_per_block";

pub const LABEL_SUCCESSFUL_TX: &str = "grug.app.successful_tx_count";

pub const LABEL_FAILED_TX: &str = "grug.app.failed_tx_count";

pub const LABEL_PROCESSED_MSGS: &str = "grug.app.processed_msgs_count";

pub const LABEL_PROCESSED_QUERIES: &str = "grug.app.processed_queries_count";

pub const LABEL_DURATION_BLOCK: &str = "grug.app.block.duration";

pub const LABEL_DURATION_TX: &str = "grug.app.tx.duration";

pub const LABEL_DURATION_PREPARE_PROPOSAL: &str = "grug.app.prepare_proposal.duration";

pub const LABEL_DURATION_COMMIT: &str = "grug.app.commit.duration";

pub(crate) fn init_metrics() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_counter!(LABEL_SUCCESSFUL_TX, "Number of successful transactions");
        describe_counter!(LABEL_FAILED_TX, "Number of failed transactions");
        describe_counter!(LABEL_PROCESSED_MSGS, "Number of processed messages");
        describe_counter!(LABEL_PROCESSED_QUERIES, "Number of processed queries");
        describe_histogram!(LABEL_TX_PER_BLOCK, "Number of transactions per block");
        describe_histogram!(LABEL_DURATION_BLOCK, "Duration of finalized block");
        describe_histogram!(LABEL_DURATION_TX, "Duration of a transaction");
        describe_histogram!(
            LABEL_DURATION_PREPARE_PROPOSAL,
            "Duration of prepare proposal"
        );
        describe_histogram!(LABEL_DURATION_COMMIT, "Duration of commit");
    });
}
