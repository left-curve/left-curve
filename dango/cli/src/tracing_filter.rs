use {
    tracing::{Metadata, Subscriber, level_filters::LevelFilter, subscriber::Interest},
    tracing_subscriber::layer::{Context, Filter},
};

/// A wrapper over tracing-subscriber's `LevelFilter`, but with one twist: all
/// events emitted by either `actix_web` or `async_graphql` crate are ignored.
///
/// These two libraries each emits an event every time a GraphQL request is
/// received, which is quite noisy. Instead, we want our log to focus on actual
/// state transitions in the blockchain.
pub struct SuppressingLevelFilter {
    inner: LevelFilter,
}

impl SuppressingLevelFilter {
    pub const fn from_inner(inner: LevelFilter) -> Self {
        Self { inner }
    }
}

impl<S> Filter<S> for SuppressingLevelFilter
where
    S: Subscriber,
{
    fn enabled(&self, meta: &Metadata<'_>, ctx: &Context<'_, S>) -> bool {
        // If the event is to be suppressed, return false. Otherwise, delegate
        // to inner.
        if suppress(meta) {
            false
        } else {
            self.inner.enabled(meta, ctx)
        }
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        // If the event is to be suppressed, return `Interest::never`. Otherwise,
        // delegate to inner.
        if suppress(meta) {
            Interest::never()
        } else {
            <LevelFilter as Filter<S>>::callsite_enabled(&self.inner, meta)
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        <LevelFilter as Filter<S>>::max_level_hint(&self.inner)
    }
}

/// If the event's module path is `actix_web` or `async_graphql`, return `true`
/// indicating the filter should suppress it.
fn suppress(meta: &Metadata<'_>) -> bool {
    meta.module_path().is_some_and(|module_path| {
        module_path.starts_with("actix_web") || module_path.starts_with("async_graphql")
    })
}
