use std::sync::Once;

use tracing_subscriber::fmt::format::FmtSpan;

// The tracing subscriber can only be set once. We ensure this by using `Once`.
static TRACING: Once = Once::new();

pub fn setup_tracing_subscriber(level: tracing::Level) {
    // If you need to know where this function was called from, you can uncomment the following lines.

    // let backtrace = std::backtrace::Backtrace::capture();
    // println!("Setting up tracing subscriber with level: {:?}", level);
    // println!("Called from:\n{}", backtrace);

    TRACING.call_once(|| {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(level)
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global tracing subscriber");
    });
}
