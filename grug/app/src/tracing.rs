use tracing::Level;

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

#[derive(Clone, Copy)]
pub struct TraceOption {
    /// Tracing level to use if an action succeeded.
    pub ok_level: Level,
    /// Tracing level to use if an action fails.
    pub error_level: Level,
}

impl TraceOption {
    /// The tracing option to use under the following situtations:
    ///
    /// - `InitChain`
    /// - `FinalizeBlock`
    pub const LOUD: Self = Self {
        ok_level: Level::INFO,
        error_level: Level::WARN,
    };
    /// The tracing option to use under the following situations:
    ///
    /// - `Query`
    /// - `CheckTx`
    pub const MUTE: Self = Self {
        ok_level: Level::TRACE,
        error_level: Level::TRACE,
    };
}
