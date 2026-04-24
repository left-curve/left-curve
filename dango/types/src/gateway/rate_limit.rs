use {
    grug::{Number, NumberConst, StdResult, Timestamp, Uint128},
    std::collections::VecDeque,
};

/// Number of hourly epochs in a rate-limit window. Used for supply
/// recalculation and per-user aggregation frequency.
pub const WINDOW_SIZE: u64 = 24;

/// All-time per-user, per-denom deposit and withdrawal totals. Observational
/// only — not used in rate-limit checks. May be used in the future for
/// trust-tier logic based on long-term deposit/withdraw patterns.
#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct Movement {
    pub deposited: Uint128,
    pub withdrawn: Uint128,
}

/// Owner-granted withdrawal credit for a specific user and denom. Allows the
/// user to withdraw up to `amount` without counting against the global rate
/// limit, until `expires_at`. The `used` field tracks how much has been consumed.
#[grug::derive(Serde, Borsh)]
pub struct WithdrawalCredit {
    pub amount: Uint128,
    pub used: Uint128,
    pub expires_at: Timestamp,
}

impl WithdrawalCredit {
    /// Returns the remaining usable credit, or zero if expired.
    pub fn remaining(&self, now: Timestamp) -> StdResult<Uint128> {
        if now >= self.expires_at {
            return Ok(Uint128::ZERO);
        }
        Ok(self.amount.checked_sub(self.used)?)
    }
}

/// Global per-denom sliding window tracking outbound withdrawals.
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
