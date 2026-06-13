/// Emit a tracing event with a dynamic level.
///
/// The `tracing::event!` macro requires the level to be a constant. Use this
/// macro instead of choose the level dynamically at runtime.
///
/// Copied from:
/// <https://github.com/tokio-rs/tracing/issues/2730#issuecomment-1943022805>
#[macro_export]
macro_rules! dyn_event {
    ($level:expr, $($arg:tt)+) => {
        match $level {
            ::tracing::Level::ERROR => ::tracing::error!($($arg)+),
            ::tracing::Level::WARN  => ::tracing::warn!($($arg)+),
            ::tracing::Level::INFO  => ::tracing::info!($($arg)+),
            ::tracing::Level::DEBUG => ::tracing::debug!($($arg)+),
            ::tracing::Level::TRACE => ::tracing::trace!($($arg)+),
        }
    };
}
