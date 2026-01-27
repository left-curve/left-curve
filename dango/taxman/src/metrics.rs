// This metric used to be part of the DEX contract. For compatibility, we keep
// the `dex` in the label without changing it to `taxman`.
pub const LABEL_DURATION_STORE_VOLUME: &str = "dango.contract.dex.store_volume.duration";

pub fn init_metrics() {
    use {metrics::describe_histogram, std::sync::Once};

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_histogram!(LABEL_DURATION_STORE_VOLUME, "Time spent on storing volume");
    });
}
