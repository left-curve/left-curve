use {
    metrics::{describe_counter, describe_histogram},
    std::sync::Once,
};

static ONCE_METRICS_INITIALIZED: Once = Once::new();

pub const TOTAL_TRADES_LABEL: &str = "dango.contract.dex.total_trades";
pub const VOLUME_PER_TRADE_LABEL: &str = "dango.contract.dex.volume_per_trade";
pub const VOLUME_PER_BLOCK_LABEL: &str = "dango.contract.dex.volume_per_block";

pub(crate) fn init_metrics() {
    ONCE_METRICS_INITIALIZED.call_once(|| {
        describe_counter!(TOTAL_TRADES_LABEL, "Total trades");
        describe_histogram!(VOLUME_PER_TRADE_LABEL, "Volume per trade");
        describe_histogram!(VOLUME_PER_BLOCK_LABEL, "Volume per block");
    });
}
