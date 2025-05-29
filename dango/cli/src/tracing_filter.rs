use {
    std::str::FromStr,
    tracing::{
        Event, Level, Metadata, Subscriber, level_filters::LevelFilter,
        metadata::ParseLevelFilterError,
    },
    tracing_subscriber::{Layer, layer::Context},
};

/// A custom tracing subscriber filter that suppresses messages from `actix_web`
/// and `async_graphql`.
///
/// These two libraries each emit an `INFO` level message for every request,
/// which is quite noisy. We want our log to focus on state transitions in the
/// application instead.
pub struct CustomLevelFilter {
    max_level: LevelFilter,
}

impl<S> Layer<S> for CustomLevelFilter
where
    S: Subscriber,
{
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        let target = metadata.target();
        let level = *metadata.level();

        // Suppress messages from `actix_web` and `async_graphql` crates that
        // are lower than DEBUG level.
        if target.starts_with("actix_web") || target.starts_with("async_graphql") {
            let effective_level = if level < Level::DEBUG {
                Level::DEBUG
            } else {
                level
            };

            effective_level <= self.max_level
        } else {
            level <= self.max_level
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Pass the event down to downstream layers.
        ctx.event(event);
    }
}

impl FromStr for CustomLevelFilter {
    type Err = ParseLevelFilterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let max_level = s.parse()?;

        Ok(Self { max_level })
    }
}
