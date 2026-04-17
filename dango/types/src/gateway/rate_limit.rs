use {
    grug::{Number, NumberConst, StdResult, Uint128},
    std::collections::VecDeque,
};

/// Number of hourly epochs in a rate-limit window. Used for supply
/// recalculation and per-user aggregation frequency.
pub const WINDOW_SIZE: u64 = 24;

/// Tracks deposited, withdrawn, and credit-used amounts within a period.
///
/// Deposit credit is computed as `deposited - credit_used`, NOT
/// `deposited - withdrawn`. This avoids double-counting: if a user withdraws
/// first (charged to global) and deposits later, the earlier withdrawal doesn't
/// consume the new deposit credit.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct Movement {
    pub deposited: Uint128,
    pub withdrawn: Uint128,
    /// How much of `deposited` has been consumed as deposit credit.
    pub credit_used: Uint128,
}

impl Movement {
    /// Returns how much deposit credit the user can still use.
    pub fn remaining_credit(&self) -> Uint128 {
        self.deposited.saturating_sub(self.credit_used)
    }

    /// Merges `other` into `self` by adding all fields.
    pub fn accumulate(&mut self, other: &Movement) -> StdResult<()> {
        self.deposited.checked_add_assign(other.deposited)?;
        self.withdrawn.checked_add_assign(other.withdrawn)?;
        self.credit_used.checked_add_assign(other.credit_used)?;
        Ok(())
    }
}

/// Per-user, per-denom movement tracking across epochs.
///
/// `current` covers a full day (`WINDOW_SIZE` epochs). It is folded into
/// `cumulative` once `WINDOW_SIZE` epochs have elapsed since `last_epoch`.
#[grug::derive(Serde, Borsh)]
pub struct UserMovement {
    pub last_epoch: u64,
    pub cumulative: Movement,
    pub current: Movement,
}

impl UserMovement {
    pub fn new(epoch: u64) -> Self {
        Self {
            last_epoch: epoch,
            cumulative: Movement::default(),
            current: Movement::default(),
        }
    }

    /// If at least `WINDOW_SIZE` epochs have passed since `last_epoch`, fold
    /// `current` into `cumulative` and reset for the new window.
    pub fn rotate_if_needed(&mut self, current_epoch: u64) -> StdResult<()> {
        if current_epoch.saturating_sub(self.last_epoch) >= WINDOW_SIZE {
            self.cumulative.accumulate(&self.current)?;
            self.current = Movement::default();
            self.last_epoch = current_epoch;
        }
        Ok(())
    }
}

/// Global per-denom sliding window tracking non-deposit-backed outbound.
///
/// The window has `WINDOW_SIZE` hourly slots. `total_24h` caches the sum of
/// all slots so the rate-limit check is O(1). On each hourly cron, the oldest
/// slot is popped (subtracted from `total_24h`) and a fresh zero slot is pushed.
#[grug::derive(Serde, Borsh)]
pub struct GlobalOutbound {
    /// One slot per hourly epoch (index 0 = current hour).
    pub window: VecDeque<Uint128>,
    /// Cached sum of all slots in `window`.
    pub total_24h: Uint128,
}

impl Default for GlobalOutbound {
    fn default() -> Self {
        Self {
            window: VecDeque::from([Uint128::ZERO]),
            total_24h: Uint128::ZERO,
        }
    }
}

impl GlobalOutbound {
    /// Returns the rolling 24h outbound (cached total, always up-to-date).
    pub fn rolling_outbound(&self) -> Uint128 {
        self.total_24h
    }

    /// Adds `amount` to the current hour's slot and updates the cached total.
    pub fn add_to_current(&mut self, amount: Uint128) {
        if let Some(current) = self.window.front_mut() {
            *current = current.saturating_add(amount);
        }
        self.total_24h = self.total_24h.saturating_add(amount);
    }

    /// Rotates the window: pushes a fresh zero slot for the new hour. Once the
    /// window has reached `WINDOW_SIZE`, the oldest slot is popped and its value
    /// subtracted from `total_24h`. O(1).
    pub fn rotate(&mut self) {
        if self.window.len() >= WINDOW_SIZE as usize
            && let Some(oldest) = self.window.pop_back()
        {
            self.total_24h = self.total_24h.saturating_sub(oldest);
        }
        self.window.push_front(Uint128::ZERO);
    }
}
