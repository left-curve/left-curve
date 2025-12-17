use {metrics::describe_histogram, std::sync::Once};

pub const LABEL_PRICE: &str = "dango.contract.oracle.price";

pub fn init_metrics() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_histogram!(LABEL_PRICE, "Price of token");
    });
}
