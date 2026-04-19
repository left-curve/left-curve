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
/// `historical` once `WINDOW_SIZE` epochs have elapsed since `last_epoch`.
#[grug::derive(Serde, Borsh)]
pub struct UserMovement {
    pub last_epoch: u64,
    /// Historical aggregate of all past windows. Currently observational only
    /// (exposed via queries, not used in rate-limit checks). May be used in the
    /// future for trust-tier logic based on long-term deposit/withdraw patterns.
    pub historical: Movement,
    pub current: Movement,
}

impl UserMovement {
    pub fn new(epoch: u64) -> Self {
        Self {
            last_epoch: epoch,
            historical: Movement::default(),
            current: Movement::default(),
        }
    }

    /// If at least `WINDOW_SIZE` epochs have passed since `last_epoch`, fold
    /// `current` into `historical` and reset for the new window. This handles
    /// arbitrary gaps: if the user was inactive for multiple windows, `current`
    /// is folded once (it already contains everything since `last_epoch`).
    pub fn rotate_if_needed(&mut self, current_epoch: u64) -> StdResult<()> {
        if current_epoch.saturating_sub(self.last_epoch) >= WINDOW_SIZE {
            self.historical.accumulate(&self.current)?;
            self.current = Movement::default();
            self.last_epoch = current_epoch;
        }
        Ok(())
    }

    /// Records a withdrawal: adds `credit_used` to the deposit credit consumed
    /// and `total` to the total withdrawn for this epoch.
    pub fn record_withdrawal(&mut self, credit_used: Uint128, total: Uint128) -> StdResult<()> {
        self.current.credit_used.checked_add_assign(credit_used)?;
        self.current.withdrawn.checked_add_assign(total)?;
        Ok(())
    }

    /// Returns how much deposit credit the user can still use in this epoch.
    pub fn remaining_credit(&self) -> Uint128 {
        self.current
            .deposited
            .saturating_sub(self.current.credit_used)
    }

    /// Splits `amount` into the portion covered by the current epoch's deposit
    /// credit and the excess that must be checked against the global rate limit.
    pub fn compute_credit_coverage(&self, amount: Uint128) -> (Uint128, Uint128) {
        let credit = self.remaining_credit();
        let free_amount = credit.min(amount);
        let excess = amount.saturating_sub(free_amount);
        (free_amount, excess)
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
    /// Cached sum of all slots in `window`. Mutate only via `add_to_current()`
    /// and `rotate()` to keep it in sync.
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
    /// Adds `amount` to the current hour's slot and updates the cached total.
    ///
    /// Invariant: `window` always has at least one slot — `Default` starts
    /// with `[0]` and `rotate` always pushes before (optionally) popping.
    /// Indexing with `[0]` so a violated invariant panics rather than
    /// silently desynchronising `total_24h` from the window contents.
    pub fn add_to_current(&mut self, amount: Uint128) -> StdResult<()> {
        self.window[0].checked_add_assign(amount)?;
        self.total_24h.checked_add_assign(amount)?;
        Ok(())
    }

    /// Rotates the window: pushes a fresh zero slot for the new hour. Once the
    /// window has reached `WINDOW_SIZE`, the oldest slot is popped and its value
    /// subtracted from `total_24h`. O(1).
    pub fn rotate(&mut self) -> StdResult<()> {
        if self.window.len() >= WINDOW_SIZE as usize
            && let Some(oldest) = self.window.pop_back()
        {
            self.total_24h.checked_sub_assign(oldest)?;
        }
        self.window.push_front(Uint128::ZERO);
        Ok(())
    }
}
