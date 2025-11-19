use opentelemetry_sdk::trace::SdkTracerProvider;
use std::sync::OnceLock;

static PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

pub fn set_provider(provider: SdkTracerProvider) {
    // Ignore if already set; first set wins.
    let _ = PROVIDER.set(provider);
}

pub fn shutdown() {
    if let Some(p) = PROVIDER.get() {
        let _ = p.shutdown();
    }
}
