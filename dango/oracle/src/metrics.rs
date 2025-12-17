pub const LABEL_PRICE: &str = "dango.contract.oracle.price";

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    use {metrics::describe_histogram, std::sync::Once};

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_histogram!(LABEL_PRICE, "Price of token");
    });
}
