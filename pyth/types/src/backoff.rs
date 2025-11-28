use std::time::Duration;

pub struct ExponentialBackoff {
    initial: Duration,
    max: Duration,
    factor: u32,
    max_attempts: Option<u32>,
    attempts: u32,
    current_backoff: Duration,
}

impl ExponentialBackoff {
    /// Create a new `ExponentialBackoff` instance.
    pub fn new(initial: Duration, max: Duration, factor: u32, max_attempts: Option<u32>) -> Self {
        Self {
            initial,
            max,
            factor,
            attempts: 0,
            max_attempts,
            current_backoff: initial,
        }
    }

    /// Get the next backoff duration.
    /// Returns `None` if the maximum number of attempts has been reached.
    pub fn next_delay(&mut self) -> Option<Duration> {
        if let Some(max_attempts) = self.max_attempts
            && self.attempts >= max_attempts
        {
            return None;
        }

        if self.current_backoff == self.max {
            return Some(self.max);
        }

        self.current_backoff = self.max.min(self.initial * self.factor.pow(self.attempts));
        self.attempts += 1;

        Some(self.current_backoff)
    }

    /// Reset the backoff to the initial state.
    pub fn reset(&mut self) {
        self.attempts = 0;
        self.current_backoff = self.initial;
    }

    /// Get the number of attempts since the last reset.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}
