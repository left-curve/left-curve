use {
    std::sync::Once,
    tracing_subscriber::{EnvFilter, FmtSubscriber},
};

// The tracing subscriber can only be set once. We ensure this by using `Once`.
static TRACING: Once = Once::new();

pub fn setup_tracing_subscriber(level: tracing::Level) {
    TRACING.call_once(|| {
        let filter = EnvFilter::new(level.to_string()); // Apply level to all crates

        let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global tracing subscriber");
    });
}
