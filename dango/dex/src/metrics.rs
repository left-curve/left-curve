use {
    metrics::{describe_counter, describe_histogram},
    std::sync::Once,
};

pub const LABEL_TRADES: &str = "dango.contract.dex.trades";
pub const LABEL_ORDERS_FILLED: &str = "dango.contract.dex.orders_filled";
pub const LABEL_TRADES_PER_BLOCK: &str = "dango.contract.dex.trades_per_block";
pub const LABEL_VOLUME_PER_TRADE: &str = "dango.contract.dex.volume_per_trade";
pub const LABEL_VOLUME_PER_BLOCK: &str = "dango.contract.dex.volume_per_block";

pub fn init_metrics() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_counter!(LABEL_TRADES, "Number of trades executed");

        describe_counter!(LABEL_ORDERS_FILLED, "Number of unique orders filled");

        describe_histogram!(LABEL_TRADES_PER_BLOCK, "Number of trades in a block");

        describe_histogram!(LABEL_VOLUME_PER_TRADE, "Volume per trade");

        describe_histogram!(LABEL_VOLUME_PER_BLOCK, "Volume per block");
    });
}
