use opentelemetry_sdk::trace::SdkTracerProvider;
use sentry::Hub;
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

pub fn shutdown_sentry() {
    if let Some(client) = Hub::current().client() {
        // Close drains pending events and shuts down the transport.
        let _ = client.close(None);
    }
}
