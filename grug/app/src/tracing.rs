use tracing::Level;

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
