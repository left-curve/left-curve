use {
    anyhow::{anyhow, ensure},
    dango_order_book::Dimensionless,
    grug_math::MathResult,
    grug_types::Timestamp,
    std::collections::VecDeque,
};

/// A single Pyth Lazer price feed subscription (feed id + channel). Identical in
/// shape to the upstream type, so we alias it directly.
pub type PriceSource = pyth_types::PythLazerSubscriptionDetails;

/// How a denom's price is derived from one or more Pyth feeds.
///
/// Most assets are priced from a single feed. Commodities priced off futures
/// markets (e.g. WTI, Brent, natural gas) instead roll between two contracts —
/// the front-month and the next-month — blending their prices over a set of
/// discrete fixings as the front contract approaches maturity. The blend weight
/// is a pure function of the block timestamp, so the roll runs on-chain without
/// a transaction per fixing.
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
/// crosses each fixing time; before the first fixing the price is 100% `current`,
/// and the last fixing must carry the full weight (100% `next`). Once the final
/// fixing has passed, the maintenance cron advances the roll by pulling the
/// following contract from `upcoming`. This mirrors trade.xyz's per-session step
/// roll (no intraday interpolation).
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
    /// Future contracts and their fixings, pre-loaded by the owner and consumed
    /// one per completed roll by the maintenance cron. Keeping the schedule
    /// on-chain is what lets the roll run without recurring owner transactions.
    pub upcoming: VecDeque<ScheduledRoll>,
}

/// A single roll fixing: at or after `at`, the weight on the next contract
/// becomes `next_weight`.
#[grug_types::derive(Serde)]
pub struct Fixing {
    pub at: Timestamp,
    pub next_weight: Dimensionless,
}

/// A pre-scheduled future roll: the contract to roll into, and the fixings for
/// rolling out of the previous `next` and into this `contract`.
#[grug_types::derive(Serde)]
pub struct ScheduledRoll {
    pub contract: PriceSource,
    pub fixings: Vec<Fixing>,
}

/// An update to a denom's `upcoming` roll schedule, applied by the chain owner
/// via `ExecuteMsg::UpdateRollSchedules`. The resulting config is always
/// re-validated, so a bad schedule is rejected.
#[grug_types::derive(Serde)]
pub enum RollScheduleUpdate {
    /// Append these scheduled rolls to the end of the existing `upcoming` queue —
    /// the routine top-up that keeps a roll running indefinitely.
    Append(Vec<ScheduledRoll>),
    /// Replace the entire `upcoming` queue with these scheduled rolls — used to
    /// correct a queue that was populated incorrectly.
    Override(Vec<ScheduledRoll>),
}

impl PriceConfig {
    /// Construct a single-feed config (the common case).
    pub fn single(source: PriceSource) -> Self {
        PriceConfig::Single(source)
    }

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

    /// Every feed this config can reference, for feed subscription. Includes the
    /// current and next contracts plus all upcoming ones, so each contract's
    /// feed is already live by the time the roll advances into it.
    pub fn feeds(&self) -> Vec<PriceSource> {
        match self {
            PriceConfig::Single(source) => vec![source.clone()],
            PriceConfig::Roll(roll) => {
                let mut feeds = vec![roll.current.clone(), roll.next.clone()];
                feeds.extend(roll.upcoming.iter().map(|s| s.contract.clone()));
                feeds
            },
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

    /// Whether the current roll has fully completed (the last fixing has passed)
    /// and a following contract is queued — i.e. the maintenance cron should
    /// call [`advance`](Self::advance).
    pub fn should_advance(&self, now: Timestamp) -> bool {
        !self.upcoming.is_empty() && self.fixings.last().is_some_and(|f| f.at <= now)
    }

    /// Advance to the next scheduled roll: the contract we just rolled into
    /// becomes the new front, and the head of `upcoming` becomes the new next.
    pub fn advance(&mut self) -> anyhow::Result<()> {
        let scheduled = self
            .upcoming
            .pop_front()
            .ok_or_else(|| anyhow!("no scheduled roll to advance into"))?;

        self.current = std::mem::replace(&mut self.next, scheduled.contract);
        self.fixings = scheduled.fixings;

        Ok(())
    }

    /// Validate the roll:
    /// - `current` and `next` must be different feeds;
    /// - `fixings` must be non-empty and strictly ascending in both time and
    ///   weight, with each weight in `(0, 1]` and the last weight exactly one;
    /// - the `upcoming` chain must be chronologically ordered (each scheduled
    ///   roll starts after the previous roll's last fixing) with each pair of
    ///   consecutive contracts different.
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.current.id != self.next.id,
            "roll `current` and `next` must be different feeds"
        );

        validate_fixings(&self.fixings)?;

        // Walk the upcoming chain `next -> upcoming[0] -> upcoming[1] -> ...`,
        // checking each roll's fixings, that consecutive contracts differ, and
        // that each roll starts strictly after the previous one's last fixing.
        let mut prev_end = self.fixings.last().map(|f| f.at);
        let mut prev_contract = self.next.id;
        for scheduled in &self.upcoming {
            validate_fixings(&scheduled.fixings)?;

            ensure!(
                scheduled.contract.id != prev_contract,
                "consecutive roll contracts must differ, got feed `{}` twice",
                scheduled.contract.id
            );

            // `validate_fixings` guarantees the list is non-empty.
            let first = scheduled.fixings.first().unwrap().at;
            if let Some(prev_end) = prev_end {
                ensure!(
                    first > prev_end,
                    "each scheduled roll must start after the previous roll's last fixing"
                );
            }

            prev_end = scheduled.fixings.last().map(|f| f.at);
            prev_contract = scheduled.contract.id;
        }

        Ok(())
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
    use {super::*, pyth_types::Channel};

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
            upcoming: VecDeque::new(),
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
        let config = PriceConfig::single(src(7));
        assert_eq!(
            config.components_at(Timestamp::from_seconds(123)).unwrap(),
            vec![(src(7), Dimensionless::ONE),]
        );
    }

    #[test]
    fn single_serializes_externally_tagged() {
        use grug_types::JsonSerExt;
        // Confirms the on-chain/genesis JSON shape of a single-source config.
        assert_eq!(
            PriceConfig::single(src(1)).to_json_string().unwrap(),
            r#"{"single":{"id":1,"channel":"real_time"}}"#
        );
    }

    #[test]
    fn validate_accepts_a_well_formed_roll() {
        PriceConfig::Roll(five_step_roll()).validate().unwrap();
        PriceConfig::single(src(1)).validate().unwrap();
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

    #[test]
    fn validate_checks_upcoming_chain() {
        let mut roll = five_step_roll();
        roll.upcoming.push_back(ScheduledRoll {
            contract: src(3),
            fixings: vec![fixing(1_000, 100)],
        });
        // Valid: the queued roll starts after the current roll's last fixing
        // (t=500) and rolls into a new contract.
        PriceConfig::Roll(roll.clone()).validate().unwrap();

        // Out of order: the queued roll starts before the current roll ends.
        let mut bad = roll.clone();
        bad.upcoming[0].fixings = vec![fixing(400, 100)];
        assert!(
            PriceConfig::Roll(bad)
                .validate()
                .unwrap_err()
                .to_string()
                .contains("after the previous")
        );

        // Repeated contract: the queued contract equals the current `next`.
        let mut bad = roll;
        bad.upcoming[0].contract = src(2);
        assert!(
            PriceConfig::Roll(bad)
                .validate()
                .unwrap_err()
                .to_string()
                .contains("must differ")
        );
    }

    #[test]
    fn advance_promotes_next_and_pulls_from_upcoming() {
        let mut roll = five_step_roll();
        roll.upcoming.push_back(ScheduledRoll {
            contract: src(3),
            fixings: vec![fixing(1_000, 100)],
        });

        // The last fixing (t=500) has passed at t=600, and a roll is queued.
        assert!(roll.should_advance(Timestamp::from_seconds(600)));

        roll.advance().unwrap();

        // `next` (2) became the new front; the queued contract (3) is the new next.
        assert_eq!(roll.current, src(2));
        assert_eq!(roll.next, src(3));
        assert_eq!(roll.fixings, vec![fixing(1_000, 100)]);
        assert!(roll.upcoming.is_empty());

        // The new fixings are in the future, so we should not advance again.
        assert!(!roll.should_advance(Timestamp::from_seconds(600)));
    }
}
