use {
    metrics::{describe_counter, describe_histogram},
    std::sync::Once,
};

static ONCE_METRICS_INITIALIZED: Once = Once::new();

pub const TOTAL_TRADES_LABEL: &str = "dango.contract.dex.total_trades";
pub const TOTAL_FILLED_ORDERS_LABEL: &str = "dango.contract.dex.total_filled_orders";
pub const TRADE_PER_BLOCK_LABEL: &str = "dango.contract.dex.trade_per_block";
pub const VOLUME_PER_TRADE_LABEL: &str = "dango.contract.dex.volume_per_trade";
pub const VOLUME_PER_BLOCK_LABEL: &str = "dango.contract.dex.volume_per_block";

pub fn init_metrics() {
    ONCE_METRICS_INITIALIZED.call_once(|| {
        describe_counter!(TOTAL_TRADES_LABEL, "Cumulative total trades");
        describe_counter!(
            TOTAL_FILLED_ORDERS_LABEL,
            "Cumulative total filled unique orders"
        );
        describe_histogram!(TRADE_PER_BLOCK_LABEL, "Trade per block");
        describe_histogram!(VOLUME_PER_TRADE_LABEL, "Volume per trade");
        describe_histogram!(VOLUME_PER_BLOCK_LABEL, "Volume per block");
    });
}
