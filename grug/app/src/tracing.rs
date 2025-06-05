#[derive(Clone, Copy)]
pub enum Level {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

#[cfg(feature = "tracing")]
impl From<Level> for tracing::Level {
    fn from(level: Level) -> Self {
        match level {
            Level::TRACE => tracing::Level::TRACE,
            Level::DEBUG => tracing::Level::DEBUG,
            Level::INFO => tracing::Level::INFO,
            Level::WARN => tracing::Level::WARN,
            Level::ERROR => tracing::Level::ERROR,
        }
    }
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
