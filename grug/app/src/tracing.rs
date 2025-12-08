#[derive(Clone, Copy)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[cfg(feature = "tracing")]
impl From<Level> for tracing::Level {
    fn from(level: Level) -> Self {
        match level {
            Level::Trace => tracing::Level::TRACE,
            Level::Debug => tracing::Level::DEBUG,
            Level::Info => tracing::Level::INFO,
            Level::Warn => tracing::Level::WARN,
            Level::Error => tracing::Level::ERROR,
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
        ok_level: Level::Info,
        error_level: Level::Warn,
    };
    /// The tracing option to use under the following situations:
    ///
    /// - `Query`
    /// - `CheckTx`
    pub const MUTE: Self = Self {
        ok_level: Level::Trace,
        error_level: Level::Trace,
    };
}
