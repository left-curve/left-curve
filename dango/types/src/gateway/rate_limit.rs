use grug::{Number, Uint128};

/// Tracks deposited, withdrawn, and credit-used amounts within an epoch.
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
    /// Returns how much deposit credit the user can still use this epoch.
    pub fn remaining_credit(&self) -> Uint128 {
        self.deposited.saturating_sub(self.credit_used)
    }

    /// Merges `other` into `self` by adding all fields.
    pub fn accumulate(&mut self, other: &Movement) {
        self.deposited = self.deposited.saturating_add(other.deposited);
        self.withdrawn = self.withdrawn.saturating_add(other.withdrawn);
        self.credit_used = self.credit_used.saturating_add(other.credit_used);
    }
}

/// Per-user movement tracking across epochs.
///
/// - `last_epoch`: the epoch when this user last interacted.
/// - `cumulative`: sum of all movements from past epochs.
/// - `current`: movements in the current epoch.
///
/// When the user interacts and the epoch has advanced, `current` is folded into
/// `cumulative` and reset.
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

    /// If the current epoch has advanced past `last_epoch`, fold `current` into
    /// `cumulative` and reset `current` for the new epoch.
    pub fn rotate_if_needed(&mut self, current_epoch: u64) {
        if current_epoch != self.last_epoch {
            self.cumulative.accumulate(&self.current);
            self.current = Movement::default();
            self.last_epoch = current_epoch;
        }
    }
}
