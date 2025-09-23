use {
    metrics::{describe_counter, describe_histogram},
    std::sync::Once,
};

pub const LABEL_TRADES: &str = "dango.contract.dex.trades_count";

pub const LABEL_ORDERS_FILLED: &str = "dango.contract.dex.orders_filled_count";

pub const LABEL_TRADES_PER_BLOCK: &str = "dango.contract.dex.trades_per_block";

pub const LABEL_VOLUME_PER_TRADE: &str = "dango.contract.dex.volume_per_trade";

pub const LABEL_VOLUME_PER_BLOCK: &str = "dango.contract.dex.volume_per_block";

pub const LABEL_DURATION_AUCTION: &str = "dango.contract.dex.duration.auction";

pub const LABEL_DURATION_REFLECT_CURVE: &str = "dango.contract.dex.duration.reflect_curve";

pub const LABEL_DURATION_ORDER_MATCHING: &str = "dango.contract.dex.duration.order_matching";

pub const LABEL_DURATION_ORDER_FILLING: &str = "dango.contract.dex.duration.order_filling";

pub const LABEL_DURATION_HANDLE_FILLED: &str = "dango.contract.dex.duration.handle_filled";

pub const LABEL_DURATION_CANCEL_IOC: &str = "dango.contract.dex.duration.cancel_ioc";

pub const LABEL_DURATION_UPDATE_REST_STATE: &str = "dango.contract.dex.duration.update_rest_state";

pub fn init_metrics() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_counter!(LABEL_TRADES, "Number of trades executed");

        describe_counter!(LABEL_ORDERS_FILLED, "Number of unique orders filled");

        describe_histogram!(LABEL_TRADES_PER_BLOCK, "Number of trades in a block");

        describe_histogram!(LABEL_VOLUME_PER_TRADE, "Volume per trade");

        describe_histogram!(LABEL_VOLUME_PER_BLOCK, "Volume per block");

        describe_histogram!(
            LABEL_DURATION_AUCTION,
            "Time spent on the entire auction across all pairs"
        );

        describe_histogram!(
            LABEL_DURATION_REFLECT_CURVE,
            "Time spent on reflecting passive liquidity pool curve"
        );

        describe_histogram!(
            LABEL_DURATION_ORDER_MATCHING,
            "Time spent on matching orders"
        );

        describe_histogram!(LABEL_DURATION_ORDER_FILLING, "Time spent on filling orders");

        describe_histogram!(
            LABEL_DURATION_HANDLE_FILLED,
            "Time spent on handling filled orders"
        );

        describe_histogram!(
            LABEL_DURATION_HANDLE_FILLED,
            "Time spent on canceling IOC orders"
        );

        describe_histogram!(
            LABEL_DURATION_UPDATE_REST_STATE,
            "Time spent on updating the resting order book state"
        );
    });
}
