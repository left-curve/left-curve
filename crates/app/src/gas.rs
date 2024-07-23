use {
    crate::Shared,
    std::{fmt, fmt::Display},
};

// ---------------------------------- config -----------------------------------

/// The default gas costs.
///
/// For now, we make this a constant. In the future we can consider making this
/// an on-chain parameter configurable by governance.
pub const GAS_COSTS: GasCosts = GasCosts {
    // Storage.
    //
    // For storage, we take the values from Cosmos SDK:
    // https://github.com/cosmos/cosmos-sdk/blob/v0.50.7/store/types/gas.go#L232-L242
    //
    // Following the conversion:
    // - 1 Cosmos SDK gas = 100 CosmWasm gas
    // - 170 CosmWasm gas = 1 Wasmer point
    // - 1 Wasmer point = 1 Grug gas
    // This means: 1 Cosmos SDK gas = 0.588 Grug gas
    db_read: LinearGasCost::new(588, 2),
    db_scan: 588,
    db_next: 18,
    db_write: LinearGasCost::new(1176, 18),
    db_remove: 588,
    // Verifiers
    //
    // For batch verification, there's a flat setup cost, and a cost per signature.
    secp256r1_verify: 1_880_000,
    secp256k1_verify: 770_000,
    secp256k1_pubkey_recover: 1_580_000,
    ed25519_verify: 410_000,
    ed25519_batch_verify: LinearGasCost::new(1_340_000, 188_000),
    // Hashers.
    //
    // For hashers, `per_item` means per byte.
    // The truncated versions have the same cost as the untruncated counterparts.
    sha2_256: LinearGasCost::new(0, 27),
    sha2_512: LinearGasCost::new(0, 16),
    sha2_512_truncated: LinearGasCost::new(0, 16),
    sha3_256: LinearGasCost::new(0, 15),
    sha3_512: LinearGasCost::new(0, 28),
    sha3_512_truncated: LinearGasCost::new(0, 28),
    keccak256: LinearGasCost::new(0, 15),
    blake2s_256: LinearGasCost::new(0, 15),
    blake2b_512: LinearGasCost::new(0, 9),
    blake3: LinearGasCost::new(0, 5),
};

pub struct GasCosts {
    // Storage
    pub db_read: LinearGasCost,
    pub db_scan: u64,
    pub db_next: u64,
    pub db_write: LinearGasCost,
    pub db_remove: u64,
    // Signature verifiers
    pub secp256r1_verify: u64,
    pub secp256k1_verify: u64,
    pub secp256k1_pubkey_recover: u64,
    pub ed25519_verify: u64,
    pub ed25519_batch_verify: LinearGasCost,
    // Hashers
    pub sha2_256: LinearGasCost,
    pub sha2_512: LinearGasCost,
    pub sha2_512_truncated: LinearGasCost,
    pub sha3_256: LinearGasCost,
    pub sha3_512: LinearGasCost,
    pub sha3_512_truncated: LinearGasCost,
    pub keccak256: LinearGasCost,
    pub blake2s_256: LinearGasCost,
    pub blake2b_512: LinearGasCost,
    pub blake3: LinearGasCost,
}

pub struct LinearGasCost {
    /// The flat part of the cost, charged once per batch.
    base: u64,
    /// The cost per item, on top of the flat part.
    per_item: u64,
}

impl LinearGasCost {
    pub const fn new(base: u64, per_item: u64) -> Self {
        Self { base, per_item }
    }

    pub fn cost(&self, items: usize) -> u64 {
        self.base + self.per_item * items as u64
    }
}

// ---------------------------------- tracker ----------------------------------

// We create an error type specifically for the gas tracker, such that there's
// an linear dependency relation between error types:
// > `OutOfGasError` --> `VmError` --> `AppError`
#[derive(Debug, thiserror::Error)]
#[error("not enough gas! limit: {limit}, used: {used}, comment: {comment}")]
pub struct OutOfGasError {
    limit: u64,
    used: u64,
    comment: &'static str,
}

struct GasTrackerInner {
    // `None` means there is no gas limit. This is the case during genesis, and
    // for begin/end blockers.
    limit: Option<u64>,
    used: u64,
}

/// Tracks gas consumption; throws error if gas limit is exceeded.
#[derive(Clone)]
pub struct GasTracker {
    inner: Shared<GasTrackerInner>,
}

impl GasTracker {
    /// Create a new gas tracker, with or without a gas limit.
    pub fn new(maybe_limit: Option<u64>) -> Self {
        Self {
            inner: Shared::new(GasTrackerInner {
                limit: maybe_limit,
                used: 0,
            }),
        }
    }

    /// Create a new gas tracker without a gas limit.
    pub fn new_limitless() -> Self {
        Self {
            inner: Shared::new(GasTrackerInner {
                limit: None,
                used: 0,
            }),
        }
    }

    /// Create a new gas tracker with the given gas limit.
    pub fn new_limited(limit: u64) -> Self {
        Self {
            inner: Shared::new(GasTrackerInner {
                limit: Some(limit),
                used: 0,
            }),
        }
    }

    /// Return the gas limit. `None` if there isn't a limit.
    ///
    /// Panics if lock is poisoned.
    pub fn limit(&self) -> Option<u64> {
        self.inner.read_access().limit
    }

    /// Return the amount of gas already used.
    ///
    /// Panics if lock is poisoned.
    pub fn used(&self) -> u64 {
        self.inner.read_access().used
    }

    /// Return the amount of gas remaining. `None` if there isn't a limit.
    ///
    /// Panics if lock is poisoned.
    pub fn remaining(&self) -> Option<u64> {
        self.inner.read_with(|inner| {
            let limit = inner.limit?;
            Some(limit - inner.used)
        })
    }

    /// Consume the given amount of gas. Error if the limit is exceeded.
    ///
    /// Panics if lock is poisoned.
    pub fn consume(&self, consumed: u64, comment: &'static str) -> Result<(), OutOfGasError> {
        self.inner.write_with(|mut inner| {
            let used = inner.used + consumed;

            // If there is a limit, and the limit is exceeded, then throw error.
            if let Some(limit) = inner.limit {
                if used > limit {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(limit = inner.limit, used, comment, "Out of gas");

                    return Err(OutOfGasError {
                        limit,
                        used,
                        comment,
                    });
                }
            }

            #[cfg(feature = "tracing")]
            tracing::debug!(limit = inner.limit, consumed, comment, "Gas consumed");

            inner.used = used;

            Ok(())
        })
    }
}

impl Display for GasTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.read_with(|inner| {
            write!(
                f,
                "GasTracker {{ limit: {:?}, used: {} }}",
                inner.limit, inner.used
            )
        })
    }
}
