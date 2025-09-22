use {
    crate::LeEcdsaMessage,
    anyhow::{Result, bail},
    grug::{AddrEncoder, Binary, EncodedBytes, NonEmpty},
    std::time::Duration,
};

pub type PythId = EncodedBytes<[u8; 32], AddrEncoder>;

#[grug::derive(Serde)]
pub struct LatestVaaResponse {
    pub binary: LatestVaaBinaryResponse,
}

#[grug::derive(Serde)]
pub struct LatestVaaBinaryResponse {
    pub data: Vec<Binary>,
}

#[grug::derive(Serde)]
pub enum PriceUpdate {
    Core(NonEmpty<Vec<Binary>>),
    Lazer(NonEmpty<Vec<LeEcdsaMessage>>),
}

impl PriceUpdate {
    /// Check if the `PriceUpdate` is a Core.
    pub fn is_core(&self) -> bool {
        matches!(self, PriceUpdate::Core(_))
    }

    /// Check if the `PriceUpdate` is a Lazer.
    pub fn is_lazer(&self) -> bool {
        matches!(self, PriceUpdate::Lazer(_))
    }

    /// Try to cast `PriceUpdate` to `Core`.
    pub fn try_into_core(&self) -> Result<NonEmpty<Vec<Binary>>> {
        match self {
            PriceUpdate::Core(core) => Ok(core.clone()),
            _ => bail!("PriceUpdate is not Core"),
        }
    }

    /// Try to cast `PriceUpdate` to `Lazer`.
    pub fn try_into_lazer(&self) -> Result<NonEmpty<Vec<LeEcdsaMessage>>> {
        match self {
            PriceUpdate::Lazer(lazer) => Ok(lazer.clone()),
            _ => bail!("PriceUpdate is not Lazer"),
        }
    }
}

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
        if let Some(max_attempts) = self.max_attempts {
            if self.attempts > max_attempts {
                return None;
            }
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
