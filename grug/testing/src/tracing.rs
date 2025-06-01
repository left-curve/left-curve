use std::sync::Once;

// The tracing subscriber can only be set once. We ensure this by using `Once`.
static TRACING: Once = Once::new();

pub fn setup_tracing_subscriber(level: tracing::Level) {
    TRACING.call_once(|| {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(level)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global tracing subscriber");
    });
}
