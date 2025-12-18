pub const LABEL_TRADES: &str = "dango.contract.dex.trades_count";

pub const LABEL_ORDERS_FILLED: &str = "dango.contract.dex.orders_filled_count";

pub const LABEL_RESERVE_AMOUNT: &str = "dango.contract.dex.reserve_amount";

pub const LABEL_RESERVE_VALUE: &str = "dango.contract.dex.reserve_value";

pub const LABEL_TRADES_PER_BLOCK: &str = "dango.contract.dex.trades_per_block";

pub const LABEL_VOLUME_PER_TRADE: &str = "dango.contract.dex.volume_per_trade";

pub const LABEL_VOLUME_PER_BLOCK: &str = "dango.contract.dex.volume_per_block";

pub const LABEL_BEST_PRICE: &str = "dango.contract.dex.best_price";

pub const LABEL_SPREAD_ABSOLUTE: &str = "dango.contract.dex.spread_absolute";

pub const LABEL_SPREAD_PERCENTAGE: &str = "dango.contract.dex.spread_percentage";

pub const LABEL_DURATION_AUCTION: &str = "dango.contract.dex.auction.duration";

pub const LABEL_DURATION_REFLECT_CURVE: &str = "dango.contract.dex.reflect_curve.duration";

pub const LABEL_DURATION_ORDER_MATCHING: &str = "dango.contract.dex.order_matching.duration";

pub const LABEL_DURATION_ORDER_FILLING: &str = "dango.contract.dex.order_filling.duration";

pub const LABEL_DURATION_CANCEL_IOC: &str = "dango.contract.dex.cancel_ioc.duration";

pub const LABEL_DURATION_UPDATE_REST_STATE: &str = "dango.contract.dex.update_rest_state.duration";

pub const LABEL_DURATION_STORE_VOLUME: &str = "dango.contract.dex.store_volume.duration";

pub const LABEL_DURATION_ITER_NEXT: &str = "dango.contract.dex.iterator_next.duration";

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    use {
        metrics::{describe_counter, describe_gauge, describe_histogram},
        std::sync::Once,
    };

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_counter!(LABEL_TRADES, "Number of trades executed");

        describe_counter!(LABEL_ORDERS_FILLED, "Number of unique orders filled");

        describe_gauge!(LABEL_RESERVE_AMOUNT, "Amount of reserve");

        describe_gauge!(LABEL_RESERVE_VALUE, "Value of reserve");

        describe_histogram!(LABEL_TRADES_PER_BLOCK, "Number of trades in a block");

        describe_histogram!(LABEL_VOLUME_PER_TRADE, "Volume per trade");

        describe_histogram!(LABEL_VOLUME_PER_BLOCK, "Volume per block");

        describe_gauge!(LABEL_BEST_PRICE, "Best price available in order book");

        describe_gauge!(LABEL_SPREAD_ABSOLUTE, "Absolute spread between bid and ask");

        describe_gauge!(
            LABEL_SPREAD_PERCENTAGE,
            "Percentage spread between bid and ask"
        );

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
            LABEL_DURATION_CANCEL_IOC,
            "Time spent on canceling IOC orders"
        );

        describe_histogram!(
            LABEL_DURATION_UPDATE_REST_STATE,
            "Time spent on updating the resting order book state"
        );

        describe_histogram!(LABEL_DURATION_STORE_VOLUME, "Time spent on storing volume");

        describe_histogram!(
            LABEL_DURATION_ITER_NEXT,
            "Time spent on advancing an iterator"
        );
    });
}
