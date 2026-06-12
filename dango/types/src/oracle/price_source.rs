use {
    anyhow::ensure, dango_order_book::Dimensionless, grug_math::MathResult, grug_types::Timestamp,
};

/// A single Pyth Lazer price feed subscription (feed id + channel). Identical in
/// shape to the upstream type, so we alias it directly.
pub type PriceSource = pyth_types::PythLazerSubscriptionDetails;

/// How a denom's price is derived from one or more Pyth feeds.
///
/// Most assets are priced from a single feed. Commodities priced off futures
/// markets (e.g. WTI, Brent, natural gas) instead roll between two contracts —
/// the front-month and the next-month — blending their prices over a set of
/// discrete fixings as the front contract approaches its final trading day.
/// The blend weight is a pure function of the block timestamp, so the roll
/// runs on-chain without a transaction per fixing.
#[grug_types::derive(Serde)]
pub enum PriceConfig {
    /// Priced from a single feed. The common case (crypto, spot).
    Single(PriceSource),

    /// Priced from a two-contract futures roll, blended by the block timestamp.
    Roll(RollState),
}

/// The state of an in-progress (or not-yet-started) futures roll.
///
/// The weight on `next` steps up through `fixings` as the block timestamp
/// crosses each fixing time: before the first fixing the price is 100% `current`,
/// and after the last fixing (which carries the full weight) it is 100% `next`.
/// This mirrors trade.xyz's per-session step roll, with no intraday
/// interpolation. Rolling forward to the next pair of contracts is done by
/// re-registering the config.
#[grug_types::derive(Serde)]
pub struct RollState {
    /// The contract being rolled out of (the front month).
    pub current: PriceSource,

    /// The contract being rolled into (the next month).
    pub next: PriceSource,

    /// Discrete roll fixings, strictly ascending in time. Each entry sets the
    /// weight on `next` at or after its timestamp; the last entry must carry
    /// weight one.
    pub fixings: Vec<Fixing>,
}

/// A single roll fixing: at or after `at`, the weight on the next contract
/// becomes `next_weight`.
#[grug_types::derive(Serde)]
pub struct Fixing {
    pub at: Timestamp,
    pub next_weight: Dimensionless,
}

impl PriceConfig {
    /// The feeds to blend at `now`, paired with their weights. Always one or two
    /// entries, and the weights always sum to one. Zero-weight components are
    /// omitted, so an expired contract whose feed has gone stale is not fetched
    /// once its weight has reached zero.
    pub fn components_at(&self, now: Timestamp) -> MathResult<Vec<(PriceSource, Dimensionless)>> {
        match self {
            PriceConfig::Single(source) => Ok(vec![(source.clone(), Dimensionless::ONE)]),
            PriceConfig::Roll(roll) => roll.components_at(now),
        }
    }

    /// Every feed this config references, for feed subscription: the single
    /// source, or both the current and next contracts of a roll.
    pub fn feeds(&self) -> Vec<PriceSource> {
        match self {
            PriceConfig::Single(source) => vec![source.clone()],
            PriceConfig::Roll(roll) => vec![roll.current.clone(), roll.next.clone()],
        }
    }

    /// Validate the config; see [`RollState::validate`].
    pub fn validate(&self) -> anyhow::Result<()> {
        match self {
            PriceConfig::Single(_) => Ok(()),
            PriceConfig::Roll(roll) => roll.validate(),
        }
    }
}

impl RollState {
    /// The weight on `next` at `now`: the weight of the last fixing at or before
    /// `now`, or zero if `now` precedes the first fixing.
    pub fn next_weight_at(&self, now: Timestamp) -> Dimensionless {
        self.fixings
            .iter()
            .rev()
            .find(|f| f.at <= now)
            .map(|f| f.next_weight)
            .unwrap_or(Dimensionless::ZERO)
    }

    fn components_at(&self, now: Timestamp) -> MathResult<Vec<(PriceSource, Dimensionless)>> {
        let w_next = self.next_weight_at(now);

        Ok(if w_next.is_zero() {
            // Before the first fixing: 100% the front contract.
            vec![(self.current.clone(), Dimensionless::ONE)]
        } else if w_next == Dimensionless::ONE {
            // After the last fixing: 100% the next contract.
            vec![(self.next.clone(), Dimensionless::ONE)]
        } else {
            // Mid-roll: blend the two. The weights sum to one by construction, so
            // the querier needs no normalizing division.
            let w_current = Dimensionless::ONE.checked_sub(w_next)?;
            vec![
                (self.current.clone(), w_current),
                (self.next.clone(), w_next),
            ]
        })
    }

    /// Validate the roll:
    /// - `current` and `next` must be different feeds;
    /// - `fixings` must be non-empty and strictly ascending in both time and
    ///   weight, with each weight in `(0, 1]` and the last weight exactly one.
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.current.id != self.next.id,
            "roll `current` and `next` must be different feeds"
        );

        validate_fixings(&self.fixings)
    }
}

/// Shared fixing validation: non-empty, strictly ascending in time and weight,
/// each weight in `(0, 1]`, last weight exactly one.
fn validate_fixings(fixings: &[Fixing]) -> anyhow::Result<()> {
    ensure!(!fixings.is_empty(), "roll has no fixings");

    let mut prev: Option<&Fixing> = None;
    for fixing in fixings {
        ensure!(
            fixing.next_weight > Dimensionless::ZERO && fixing.next_weight <= Dimensionless::ONE,
            "roll fixing weight must be in (0, 1], got `{}`",
            fixing.next_weight
        );

        if let Some(prev) = prev {
            ensure!(
                fixing.at > prev.at,
                "roll fixings must be strictly ascending in time"
            );
            ensure!(
                fixing.next_weight > prev.next_weight,
                "roll fixing weights must be strictly ascending"
            );
        }

        prev = Some(fixing);
    }

    ensure!(
        fixings.last().unwrap().next_weight == Dimensionless::ONE,
        "the last roll fixing must carry full weight (1)"
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug_types::JsonSerExt, pyth_types::Channel};

    fn src(id: u32) -> PriceSource {
        PriceSource {
            id,
            channel: Channel::RealTime,
        }
    }

    fn fixing(secs: u128, weight_pct: i128) -> Fixing {
        Fixing {
            at: Timestamp::from_seconds(secs),
            next_weight: Dimensionless::new_percent(weight_pct),
        }
    }

    /// A 5-step roll: 20% per fixing, the classic trade.xyz shape.
    fn five_step_roll() -> RollState {
        RollState {
            current: src(1),
            next: src(2),
            fixings: vec![
                fixing(100, 20),
                fixing(200, 40),
                fixing(300, 60),
                fixing(400, 80),
                fixing(500, 100),
            ],
        }
    }

    #[test]
    fn next_weight_is_a_step_function_of_time() {
        let roll = five_step_roll();

        // Before the first fixing: zero weight on `next`.
        assert_eq!(
            roll.next_weight_at(Timestamp::from_seconds(50)),
            Dimensionless::ZERO
        );

        // On a fixing boundary the new weight takes effect (inclusive).
        assert_eq!(
            roll.next_weight_at(Timestamp::from_seconds(100)),
            Dimensionless::new_percent(20)
        );

        // Held flat between fixings (no interpolation).
        assert_eq!(
            roll.next_weight_at(Timestamp::from_seconds(199)),
            Dimensionless::new_percent(20)
        );
        assert_eq!(
            roll.next_weight_at(Timestamp::from_seconds(200)),
            Dimensionless::new_percent(40)
        );

        // After the last fixing: full weight, held forever.
        assert_eq!(
            roll.next_weight_at(Timestamp::from_seconds(10_000)),
            Dimensionless::ONE
        );
    }

    #[test]
    fn components_skip_zero_weight_endpoints() {
        let roll = PriceConfig::Roll(five_step_roll());

        // Before the roll: only the front contract.
        assert_eq!(
            roll.components_at(Timestamp::from_seconds(50)).unwrap(),
            vec![(src(1), Dimensionless::ONE),]
        );

        // Mid-roll at 40%: front carries 60%, next carries 40%; sum is one.
        assert_eq!(
            roll.components_at(Timestamp::from_seconds(200)).unwrap(),
            vec![
                (src(1), Dimensionless::new_percent(60)),
                (src(2), Dimensionless::new_percent(40)),
            ]
        );

        // After the roll: only the next contract (the old front is not fetched).
        assert_eq!(
            roll.components_at(Timestamp::from_seconds(600)).unwrap(),
            vec![(src(2), Dimensionless::ONE),]
        );
    }

    #[test]
    fn single_is_full_weight_at_any_time() {
        let config = PriceConfig::Single(src(7));
        assert_eq!(
            config.components_at(Timestamp::from_seconds(123)).unwrap(),
            vec![(src(7), Dimensionless::ONE),]
        );
    }

    /// Confirms the on-chain/genesis JSON shape of a single-source config.
    #[test]
    fn single_serializes_externally_tagged() {
        assert_eq!(
            PriceConfig::Single(src(1)).to_json_string().unwrap(),
            r#"{"single":{"id":1,"channel":"real_time"}}"#
        );
    }

    #[test]
    fn validate_accepts_a_well_formed_roll() {
        PriceConfig::Roll(five_step_roll()).validate().unwrap();
        PriceConfig::Single(src(1)).validate().unwrap();
    }

    #[test]
    fn validate_rejects_bad_rolls() {
        // Same feed on both legs.
        let mut roll = five_step_roll();
        roll.next = src(1);
        assert!(
            roll.validate()
                .unwrap_err()
                .to_string()
                .contains("different feeds")
        );

        // Last fixing not at full weight.
        let mut roll = five_step_roll();
        roll.fixings.last_mut().unwrap().next_weight = Dimensionless::new_percent(90);
        assert!(
            roll.validate()
                .unwrap_err()
                .to_string()
                .contains("full weight")
        );

        // Non-ascending weights.
        let mut roll = five_step_roll();
        roll.fixings[1].next_weight = Dimensionless::new_percent(20);
        assert!(
            roll.validate()
                .unwrap_err()
                .to_string()
                .contains("ascending")
        );

        // Empty fixings.
        let mut roll = five_step_roll();
        roll.fixings.clear();
        assert!(
            roll.validate()
                .unwrap_err()
                .to_string()
                .contains("no fixings")
        );
    }
}
