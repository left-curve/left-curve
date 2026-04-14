pub const LABEL_DEPOSITS: &str = "dango.contract.gateway.deposits";

pub const LABEL_WITHDRAWALS: &str = "dango.contract.gateway.withdrawals";

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    use {metrics::describe_counter, std::sync::Once};

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        describe_counter!(LABEL_DEPOSITS, "Amount deposited");

        describe_counter!(LABEL_WITHDRAWALS, "Amount withdrawn");
    });
}
