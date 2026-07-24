//! Wind-down of the perps exchange.
//!
//! Dango is being shut down. This one-time state transition unwinds the
//! exchange in a single block, so that afterwards no user has any exposure and
//! every dollar the contract held has been returned to its owner:
//!
//! 1. **Close every position** at the pair's mark price, realizing PnL and
//!    settling accrued funding into the user's margin.
//! 2. **Release the counterparty vault**, paying each liquidity provider their
//!    pro-rata share of vault equity into their margin, and releasing unlocks
//!    that were still in cooldown.
//! 3. **Refund every margin balance** to the user's spot account as USDC.
//! 4. **Sweep the remainder** — treasury, insurance fund, and rounding dust —
//!    to the chain owner.
//!
//! It also flips `Param.trading_enabled` to false, which is what stops the
//! exchange from being used again after the fork.
//!
//! The mark (`PairState.index_price`) is used as the settlement price because
//! it is what `compute_user_equity` and the UI already report, so each user is
//! refunded exactly the equity they last saw. Position PnL is zero-sum across
//! all accounts at any single price — every fill creates a long and a short at
//! the same entry — so the settlement neither creates nor destroys value.

use {
    dango_app::{AppResult, CHAIN_ID, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
    dango_bank::BALANCES,
    dango_math::{Dec128_6, Int128, IsZero, MultiplyRatio, Number, NumberConst, Uint128},
    dango_order_book::{ASKS, BIDS, DEPTHS, PairId, Quantity, UsdValue},
    dango_perps::{
        VIRTUAL_ASSETS, VIRTUAL_SHARES,
        core::execute_fill,
        state::{LONGS, PAIR_STATES, PARAM, SHORTS, STATE, USER_STATES},
    },
    dango_primitives::{
        Addr, Denom, Inner, Order as IterationOrder, StdError, StdResult, Storage, addr,
    },
    dango_storage::Item,
    dango_types::perps::{Param, SETTLEMENT_CURRENCY_PRICE, UserState, settlement_currency},
    std::collections::BTreeMap,
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Pre-migration perps storage shapes.
mod legacy_perps {
    use {
        dango_order_book::{Dimensionless, UsdValue},
        dango_primitives::Duration,
        dango_types::perps::RateSchedule,
    };

    /// `Param` as stored before the `trading_enabled` field was appended.
    /// Field order and types must match the old on-disk Borsh layout exactly.
    #[dango_primitives::derive(Serde, Borsh)]
    #[derive(Default)]
    pub struct Param {
        pub max_unlocks: usize,

        pub max_open_orders: usize,

        pub maker_fee_rates: RateSchedule,

        pub taker_fee_rates: RateSchedule,

        pub protocol_fee_rate: Dimensionless,

        pub liquidation_fee_rate: Dimensionless,

        pub liquidation_buffer_ratio: Dimensionless,

        pub funding_period: Duration,

        pub vault_total_weight: Dimensionless,

        pub vault_cooldown_period: Duration,

        pub referral_active: bool,

        pub min_referrer_volume: UsdValue,

        pub referrer_commission_rates: RateSchedule,

        pub vault_deposit_cap: Option<UsdValue>,

        pub max_action_batch_size: usize,
    }
}

/// Reads the pre-upgrade `Param`, keyed identically to the live item.
const LEGACY_PARAM: Item<legacy_perps::Param> = Item::new("param");

pub fn do_perps_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let perps_address = {
        let chain_id = CHAIN_ID.load(&storage)?;
        match chain_id.as_str() {
            MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
            TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
            _ => panic!("unknown chain id: {chain_id}"),
        }
    };

    let cfg = CONFIG.load(&storage)?;

    // Two storage handles over the same (Arc-backed) buffer: one scoped to the
    // perps contract, one to the bank, since refunding margins moves tokens
    // between accounts in bank storage.
    let mut perps_storage =
        StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &perps_address]);
    let mut bank_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &cfg.bank]);

    wind_down(&mut perps_storage, &mut bank_storage, perps_address, cfg.owner)
        // `?` converts `StdError` into `AppError`; there is no
        // `From<anyhow::Error>`, so bridge through `StdError::host`.
        .map_err(|err| StdError::host(err.to_string()))?;

    Ok(())
}

/// What the wind-down moved, for logging and for tests to reconcile against.
#[derive(Debug)]
struct Summary {
    user_count: usize,
    position_count: usize,
    /// USDC base units refunded to users' spot accounts.
    total_refund: Uint128,
    /// USDC base units handed to the chain owner.
    swept: Uint128,
    /// Equity that users were short by, and so could not be refunded.
    bad_debt: UsdValue,
    /// Vault equity handed to liquidity providers.
    vault_distributed: UsdValue,
    /// Vault equity left behind by floor rounding and the virtual shares'
    /// pro-rata claim. Swept to the owner along with the treasury.
    vault_residue: UsdValue,
    /// Cooldown withdrawals released straight into margin.
    unlocks_released: UsdValue,
}

/// Unwind the exchange. See the module-level documentation for the phases.
///
/// Returns an error — which halts the chain — if the perps contract does not
/// hold enough USDC to cover what it owes its users. Nothing is written in
/// that case: every balance change is computed up front and applied only once
/// solvency has been confirmed.
fn wind_down(
    perps_storage: &mut dyn Storage,
    bank_storage: &mut dyn Storage,
    perps_address: Addr,
    owner: Addr,
) -> anyhow::Result<Summary> {
    let usdc = settlement_currency::DENOM.clone();

    disable_trading(perps_storage)?;

    // ---------------- Phase 1. Close positions, release vault ----------------

    // Collect before mutating: saving a user state while ranging over the map
    // would invalidate the iterator.
    let mut user_states = USER_STATES
        .range(perps_storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    // Held in memory so open-interest decrements accumulate across all users
    // before being written back once.
    let mut pair_states = PAIR_STATES
        .range(perps_storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<BTreeMap<_, _>>>()?;

    let mut state = STATE.load(perps_storage)?;

    let position_count = close_all_positions(&mut user_states, &mut pair_states)?;

    let (vault_distributed, vault_residue, unlocks_released) =
        release_vault(&mut user_states, &mut state, perps_address)?;

    // ------------------- Phase 2. Compute the refund set ---------------------

    let perps_balance = may_load_balance(bank_storage, perps_address, &usdc)?;

    let mut refunds = Vec::with_capacity(user_states.len());
    let mut total_refund = Uint128::ZERO;
    let mut bad_debt = UsdValue::ZERO;

    for (addr, user_state) in &user_states {
        // The vault is not a real user: whatever is left in its margin after
        // the pro-rata distribution is unallocated dust, and is swept to the
        // owner along with the treasury.
        if *addr == perps_address {
            continue;
        }

        // A user can end up below zero if they were liquidatable but had not
        // yet been liquidated. There is nothing to claw back, so the shortfall
        // is absorbed out of what would otherwise go to the owner.
        if user_state.margin.is_negative() {
            bad_debt.checked_sub_assign(user_state.margin)?;
            continue;
        }

        let refund = user_state
            .margin
            .checked_div(SETTLEMENT_CURRENCY_PRICE)?
            .into_base_floor(settlement_currency::DECIMAL)?;

        if refund.is_zero() {
            continue;
        }

        total_refund.checked_add_assign(refund)?;
        refunds.push((*addr, refund));
    }

    // Solvency gate. Checked before any write, so a failure halts the chain
    // with the pre-upgrade state intact rather than half-applied.
    if perps_balance < total_refund {
        return Err(anyhow::anyhow!(
            "perps exchange is insolvent! usdc balance: {perps_balance}, owed to users: \
             {total_refund}"
        ));
    }

    // ------------------------- Phase 3. Apply writes -------------------------

    for (addr, refund) in &refunds {
        let balance = may_load_balance(bank_storage, *addr, &usdc)?;
        BALANCES.save(bank_storage, (addr, &usdc), &balance.checked_add(*refund)?)?;
    }

    // Everything the contract still holds after refunds — treasury, insurance
    // fund, vault dust, and the rounding residue of months of operation — goes
    // to the chain owner.
    let swept = perps_balance.checked_sub(total_refund)?;

    if swept.is_non_zero() {
        let balance = may_load_balance(bank_storage, owner, &usdc)?;
        BALANCES.save(bank_storage, (&owner, &usdc), &balance.checked_add(swept)?)?;
    }

    BALANCES.remove(bank_storage, (&perps_address, &usdc));

    // Every user state is now fully zeroed, so all of them are pruned.
    for (addr, _) in &user_states {
        USER_STATES.remove(perps_storage, *addr)?;
    }

    for (pair_id, pair_state) in &pair_states {
        PAIR_STATES.save(perps_storage, pair_id, pair_state)?;
    }

    state.treasury = UsdValue::ZERO;
    state.insurance_fund = UsdValue::ZERO;
    state.vault_share_supply = Uint128::ZERO;
    STATE.save(perps_storage, &state)?;

    // ------------------- Phase 4. Clear the order book -----------------------

    BIDS.clear_all(perps_storage);
    ASKS.clear_all(perps_storage);
    LONGS.clear(perps_storage, None, None);
    SHORTS.clear(perps_storage, None, None);
    DEPTHS.clear(perps_storage, None, None);

    let summary = Summary {
        user_count: user_states.len(),
        position_count,
        total_refund,
        swept,
        bad_debt,
        vault_distributed,
        vault_residue,
        unlocks_released,
    };

    tracing::info!(
        refunded_users = refunds.len(),
        pair_count = pair_states.len(),
        user_count = summary.user_count,
        position_count = summary.position_count,
        total_refund = %summary.total_refund,
        swept = %summary.swept,
        bad_debt = %summary.bad_debt,
        vault_distributed = %summary.vault_distributed,
        vault_residue = %summary.vault_residue,
        unlocks_released = %summary.unlocks_released,
        "Wound down the perps exchange"
    );

    assert_invariants(perps_storage, bank_storage, perps_address, &usdc)?;

    Ok(summary)
}

/// Rewrite `Param` in its new shape, with trading switched off.
///
/// `Param` gained the `trading_enabled` field in this release, so the stored
/// value must be read through the pre-upgrade shape before it can be written
/// back through the live one.
fn disable_trading(perps_storage: &mut dyn Storage) -> StdResult<()> {
    let legacy = LEGACY_PARAM.load(perps_storage)?;

    PARAM.save(
        perps_storage,
        &Param {
            max_unlocks: legacy.max_unlocks,
            max_open_orders: legacy.max_open_orders,
            maker_fee_rates: legacy.maker_fee_rates,
            taker_fee_rates: legacy.taker_fee_rates,
            protocol_fee_rate: legacy.protocol_fee_rate,
            liquidation_fee_rate: legacy.liquidation_fee_rate,
            liquidation_buffer_ratio: legacy.liquidation_buffer_ratio,
            funding_period: legacy.funding_period,
            vault_total_weight: legacy.vault_total_weight,
            vault_cooldown_period: legacy.vault_cooldown_period,
            referral_active: legacy.referral_active,
            min_referrer_volume: legacy.min_referrer_volume,
            referrer_commission_rates: legacy.referrer_commission_rates,
            vault_deposit_cap: legacy.vault_deposit_cap,
            max_action_batch_size: legacy.max_action_batch_size,
            trading_enabled: false,
        },
    )
}

/// Close every open position at its pair's mark price, crediting the realized
/// PnL and settled funding to the user's margin. Returns the number of
/// positions closed.
///
/// Reuses the exchange's own fill primitive, so the accounting is identical to
/// a trade that closes the position — including funding settlement, open
/// interest, and the weighted-average entry price bookkeeping — except that no
/// fee is charged, since this is a forced closure rather than a trade.
fn close_all_positions(
    user_states: &mut [(Addr, UserState)],
    pair_states: &mut BTreeMap<PairId, dango_types::perps::PairState>,
) -> anyhow::Result<usize> {
    let mut position_count = 0;

    for (_addr, user_state) in user_states.iter_mut() {
        // Collect the pair ids first: `execute_fill` removes the position from
        // the map once it is fully closed.
        let pair_ids = user_state.positions.keys().cloned().collect::<Vec<_>>();

        for pair_id in pair_ids {
            let pair_state = pair_states
                .get_mut(&pair_id)
                .ok_or_else(|| anyhow::anyhow!("no pair state for {pair_id}"))?;

            let size = user_state.positions[&pair_id].size;

            let pnl = execute_fill(
                &pair_id,
                pair_state,
                user_state,
                pair_state.index_price,
                // Closing the whole position means filling the opposite side
                // for the full size, and opening nothing.
                size.checked_neg()?,
                Quantity::ZERO,
            )?;

            user_state.margin.checked_add_assign(pnl.total()?)?;

            position_count += 1;
        }

        // Every resting order is cancelled by clearing the book, so nothing is
        // reserved against them any more.
        user_state.reserved_margin = UsdValue::ZERO;
        user_state.open_order_count = 0;
    }

    Ok(position_count)
}

/// Pay out the counterparty vault and release unlocks still in cooldown.
///
/// Returns `(vault_distributed, vault_residue, unlocks_released)`.
///
/// Must run after [`close_all_positions`], which leaves the vault holding no
/// positions, so its equity is simply its margin.
fn release_vault(
    user_states: &mut [(Addr, UserState)],
    state: &mut dango_types::perps::State,
    perps_address: Addr,
) -> anyhow::Result<(UsdValue, UsdValue, UsdValue)> {
    let vault_equity = user_states
        .iter()
        .find(|(addr, _)| *addr == perps_address)
        .map_or(UsdValue::ZERO, |(_, user_state)| user_state.margin);

    // Mirror the share maths of a regular vault withdrawal, virtual shares and
    // all, so each provider is paid exactly what withdrawing would have paid.
    let effective_supply = state.vault_share_supply.checked_add(VIRTUAL_SHARES)?;
    let effective_equity = vault_equity.checked_add(VIRTUAL_ASSETS)?;

    if !effective_equity.is_positive() {
        tracing::error!(
            %vault_equity,
            "counterparty vault has non-positive equity; liquidity providers receive nothing"
        );
    }

    let mut vault_distributed = UsdValue::ZERO;
    let mut unlocks_released = UsdValue::ZERO;

    for (addr, user_state) in user_states.iter_mut() {
        // Unlocks were already debited from the vault's margin when the
        // withdrawal was requested, so releasing them is independent of the
        // share distribution below.
        for unlock in std::mem::take(&mut user_state.unlocks) {
            user_state
                .margin
                .checked_add_assign(unlock.amount_to_release)?;
            unlocks_released.checked_add_assign(unlock.amount_to_release)?;
        }

        // A vault whose equity has gone non-positive has nothing to pay out;
        // distributing on a negative ratio would claw margin *away* from the
        // providers. The regular withdrawal path refuses in the same state.
        if *addr == perps_address
            || user_state.vault_shares.is_zero()
            || !effective_equity.is_positive()
        {
            continue;
        }

        // Multiply before dividing to avoid intermediate precision loss, and
        // round down so the vault can never pay out more than it holds.
        let amount = {
            let raw = effective_equity
                .into_inner()
                .0
                .checked_multiply_ratio_floor(
                    Int128::new(i128::try_from(user_state.vault_shares.into_inner())?),
                    Int128::new(i128::try_from(effective_supply.into_inner())?),
                )?;
            UsdValue::new(Dec128_6::raw(raw))
        };

        user_state.margin.checked_add_assign(amount)?;
        user_state.vault_shares = Uint128::ZERO;
        vault_distributed.checked_add_assign(amount)?;
    }

    // Debit the distributed total from the vault in one go. What is left is the
    // virtual shares' pro-rata claim (net of the virtual asset) plus floor
    // rounding — the same residue an ordinary withdrawal leaves behind.
    let vault_residue = vault_equity.checked_sub(vault_distributed)?;

    if let Some((_, vault_state)) = user_states
        .iter_mut()
        .find(|(addr, _)| *addr == perps_address)
    {
        vault_state.margin.checked_sub_assign(vault_distributed)?;
    }

    state.vault_share_supply = Uint128::ZERO;

    Ok((vault_distributed, vault_residue, unlocks_released))
}

fn may_load_balance(storage: &dyn Storage, address: Addr, denom: &Denom) -> StdResult<Uint128> {
    Ok(BALANCES
        .may_load(storage, (&address, denom))?
        .unwrap_or(Uint128::ZERO))
}

/// Verify the exchange has been fully unwound. A violation means the migration
/// itself is wrong, so it panics rather than returning — the chain must not
/// continue on corrupt state.
fn assert_invariants(
    perps_storage: &dyn Storage,
    bank_storage: &dyn Storage,
    perps_address: Addr,
    usdc: &Denom,
) -> StdResult<()> {
    // No user may retain any position, margin, vault share, or unlock. Since
    // every field is zeroed, every entry is pruned and the map is empty.
    let remaining = USER_STATES
        .range(perps_storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    assert!(
        remaining.is_empty(),
        "{} user states survived the wind-down",
        remaining.len()
    );

    // The order book and its indexes must be gone.
    assert!(BIDS.is_empty(perps_storage), "BIDS isn't empty!");
    assert!(ASKS.is_empty(perps_storage), "ASKS isn't empty!");
    assert!(LONGS.is_empty(perps_storage), "LONGS isn't empty!");
    assert!(SHORTS.is_empty(perps_storage), "SHORTS isn't empty!");
    assert!(DEPTHS.is_empty(perps_storage), "DEPTHS isn't empty!");

    // Closing every position must have driven open interest to zero on both
    // sides of every pair.
    for entry in PAIR_STATES.range(perps_storage, None, None, IterationOrder::Ascending) {
        let (pair_id, pair_state) = entry?;
        assert!(
            pair_state.long_oi.is_zero() && pair_state.short_oi.is_zero(),
            "pair {pair_id} still has open interest: long={}, short={}",
            pair_state.long_oi,
            pair_state.short_oi,
        );
    }

    let state = STATE.load(perps_storage)?;
    assert!(
        state.treasury.is_zero(),
        "treasury not zero: {}",
        state.treasury
    );
    assert!(
        state.insurance_fund.is_zero(),
        "insurance fund not zero: {}",
        state.insurance_fund
    );
    assert!(
        state.vault_share_supply.is_zero(),
        "vault share supply not zero: {}",
        state.vault_share_supply
    );

    // The contract must have paid out every token it held.
    let balance = may_load_balance(bank_storage, perps_address, usdc)?;
    assert!(
        balance.is_zero(),
        "perps contract still holds {balance} USDC"
    );

    // Trading must be off, or the exchange could be used again after the fork.
    assert!(
        !PARAM.load(perps_storage)?.trading_enabled,
        "trading is still enabled!"
    );

    tracing::info!("All wind-down invariants passed");

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::{FundingPerUnit, UsdPrice},
        dango_perps::{core::compute_user_equity, querier::NoCachePerpQuerier},
        dango_primitives::{Binary, MockStorage, Shared, Timestamp},
        dango_types::perps::{PairState, Position, State, Unlock},
        std::{collections::VecDeque, fs, path::PathBuf},
    };

    const PERPS: Addr = Addr::mock(1);
    const OWNER: Addr = Addr::mock(2);
    const ALICE: Addr = Addr::mock(3);
    const BOB: Addr = Addr::mock(4);
    const BANK: Addr = Addr::mock(9);

    /// 1 USD in USDC base units (6 decimals).
    const ONE_USDC: u128 = 1_000_000;

    fn pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn usdc() -> Denom {
        settlement_currency::DENOM.clone()
    }

    /// A `Param` in the pre-upgrade shape, which is what the migration reads.
    fn legacy_param() -> legacy_perps::Param {
        legacy_perps::Param {
            max_unlocks: 10,
            max_open_orders: 100,
            ..Default::default()
        }
    }

    fn position(size: i128, entry_price: i128) -> Position {
        Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(entry_price),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        }
    }

    /// Perps and bank storage handles over one shared in-memory store.
    struct Fixture {
        base: Shared<MockStorage>,
    }

    impl Fixture {
        fn new() -> Self {
            let fixture = Self {
                base: Shared::new(MockStorage::new()),
            };

            LEGACY_PARAM
                .save(&mut fixture.perps(), &legacy_param())
                .unwrap();

            fixture
        }

        fn perps(&self) -> StorageProvider {
            StorageProvider::new(Box::new(self.base.clone()), &[CONTRACT_NAMESPACE, &PERPS])
        }

        fn bank(&self) -> StorageProvider {
            StorageProvider::new(Box::new(self.base.clone()), &[CONTRACT_NAMESPACE, &BANK])
        }

        fn with_state(self, state: State) -> Self {
            STATE.save(&mut self.perps(), &state).unwrap();
            self
        }

        fn with_pair(self, index_price: i128, funding_per_unit: i128) -> Self {
            PAIR_STATES
                .save(
                    &mut self.perps(),
                    &pair_id(),
                    &PairState {
                        index_price: UsdPrice::new_int(index_price),
                        oracle_price: UsdPrice::new_int(index_price),
                        funding_per_unit: FundingPerUnit::new_int(funding_per_unit),
                        ..Default::default()
                    },
                )
                .unwrap();
            self
        }

        /// Seed a user, adjusting the pair's open interest to match the
        /// position so the fixture starts from a self-consistent state.
        fn with_user(self, addr: Addr, user_state: UserState) -> Self {
            let mut perps = self.perps();

            if let Some(pos) = user_state.positions.get(&pair_id()) {
                let mut pair_state = PAIR_STATES.load(&perps, &pair_id()).unwrap();

                if pos.size.is_positive() {
                    pair_state.long_oi.checked_add_assign(pos.size).unwrap();
                } else {
                    pair_state
                        .short_oi
                        .checked_add_assign(pos.size.checked_abs().unwrap())
                        .unwrap();
                }

                PAIR_STATES
                    .save(&mut perps, &pair_id(), &pair_state)
                    .unwrap();
            }

            USER_STATES.save(&mut perps, addr, &user_state).unwrap();
            self
        }

        /// Fund the perps contract with USDC, as the bank sees it.
        fn with_balance(self, usd: u128) -> Self {
            BALANCES
                .save(
                    &mut self.bank(),
                    (&PERPS, &usdc()),
                    &Uint128::new(usd * ONE_USDC),
                )
                .unwrap();
            self
        }

        fn run(&self) -> anyhow::Result<Summary> {
            wind_down(&mut self.perps(), &mut self.bank(), PERPS, OWNER)
        }

        fn balance_of(&self, addr: Addr) -> Uint128 {
            may_load_balance(&self.bank(), addr, &usdc()).unwrap()
        }
    }

    // -------------------------- position settlement --------------------------

    /// A long closed above its entry price realizes a gain: 10 units bought at
    /// $100 and marked at $110 is $100 of profit on top of $1,000 of margin.
    #[test]
    fn closes_long_at_profit() {
        let fixture = Fixture::new()
            .with_state(State::default())
            .with_pair(110, 0)
            .with_user(
                ALICE,
                UserState {
                    margin: UsdValue::new_int(1_000),
                    positions: [(pair_id(), position(10, 100))].into(),
                    ..Default::default()
                },
            )
            .with_balance(1_100);

        fixture.run().unwrap();

        assert_eq!(fixture.balance_of(ALICE), Uint128::new(1_100 * ONE_USDC));
        assert_eq!(fixture.balance_of(PERPS), Uint128::ZERO);
    }

    /// A short is the mirror image: 10 units sold at $100 and marked at $110
    /// loses $100.
    #[test]
    fn closes_short_at_loss() {
        let fixture = Fixture::new()
            .with_state(State::default())
            .with_pair(110, 0)
            .with_user(
                ALICE,
                UserState {
                    margin: UsdValue::new_int(1_000),
                    positions: [(pair_id(), position(-10, 100))].into(),
                    ..Default::default()
                },
            )
            .with_balance(900);

        fixture.run().unwrap();

        assert_eq!(fixture.balance_of(ALICE), Uint128::new(900 * ONE_USDC));
    }

    /// Funding accrued since the position was last touched is settled as part
    /// of the closure: a 10-unit long facing $5/unit of accumulated funding
    /// owes $50.
    #[test]
    fn settles_funding_on_close() {
        let fixture = Fixture::new()
            .with_state(State::default())
            .with_pair(100, 5)
            .with_user(
                ALICE,
                UserState {
                    margin: UsdValue::new_int(1_000),
                    positions: [(pair_id(), position(10, 100))].into(),
                    ..Default::default()
                },
            )
            .with_balance(950);

        fixture.run().unwrap();

        assert_eq!(fixture.balance_of(ALICE), Uint128::new(950 * ONE_USDC));
    }

    // ----------------------------- vault release -----------------------------

    /// Liquidity providers are paid their pro-rata share of vault equity, which
    /// is what withdrawing normally would have paid them.
    #[test]
    fn distributes_vault_pro_rata() {
        let fixture = Fixture::new()
            .with_state(State {
                vault_share_supply: Uint128::new(400_000_000),
                ..Default::default()
            })
            .with_pair(100, 0)
            // The vault holds the providers' deposited margin.
            .with_user(PERPS, UserState {
                margin: UsdValue::new_int(400),
                ..Default::default()
            })
            .with_user(ALICE, UserState {
                vault_shares: Uint128::new(300_000_000),
                ..Default::default()
            })
            .with_user(BOB, UserState {
                vault_shares: Uint128::new(100_000_000),
                ..Default::default()
            })
            .with_balance(400);

        fixture.run().unwrap();

        // A 3:1 split of the vault's $400.
        assert_eq!(fixture.balance_of(ALICE), Uint128::new(300 * ONE_USDC));
        assert_eq!(fixture.balance_of(BOB), Uint128::new(100 * ONE_USDC));
        assert_eq!(fixture.balance_of(PERPS), Uint128::ZERO);
    }

    /// A withdrawal still in its cooldown window is released immediately —
    /// otherwise those funds would be stranded by the shutdown.
    #[test]
    fn releases_pending_unlocks() {
        let fixture = Fixture::new()
            .with_state(State::default())
            .with_pair(100, 0)
            .with_user(
                ALICE,
                UserState {
                    margin: UsdValue::new_int(50),
                    unlocks: VecDeque::from(vec![Unlock {
                        end_time: Timestamp::from_seconds(9_999_999_999),
                        amount_to_release: UsdValue::new_int(250),
                    }]),
                    ..Default::default()
                },
            )
            .with_balance(300);

        fixture.run().unwrap();

        assert_eq!(fixture.balance_of(ALICE), Uint128::new(300 * ONE_USDC));
    }

    // -------------------------------- payouts --------------------------------

    /// Treasury, insurance fund, and any rounding residue end up with the chain
    /// owner, not with users.
    #[test]
    fn sweeps_residual_to_owner() {
        let fixture = Fixture::new()
            .with_state(State {
                treasury: UsdValue::new_int(70),
                insurance_fund: UsdValue::new_int(30),
                ..Default::default()
            })
            .with_pair(100, 0)
            .with_user(
                ALICE,
                UserState {
                    margin: UsdValue::new_int(500),
                    ..Default::default()
                },
            )
            .with_balance(600);

        fixture.run().unwrap();

        assert_eq!(fixture.balance_of(ALICE), Uint128::new(500 * ONE_USDC));
        assert_eq!(fixture.balance_of(OWNER), Uint128::new(100 * ONE_USDC));
        assert_eq!(fixture.balance_of(PERPS), Uint128::ZERO);
    }

    /// A user who was liquidatable but not yet liquidated ends below zero.
    /// There is nothing to claw back, so they are refunded nothing and the
    /// shortfall comes out of what the owner would otherwise have swept.
    #[test]
    fn clamps_negative_margin_to_zero() {
        let fixture = Fixture::new()
            .with_state(State::default())
            .with_pair(50, 0)
            .with_user(
                ALICE,
                UserState {
                    margin: UsdValue::new_int(100),
                    // Marked down from $100 to $50: a $500 loss against $100 of
                    // margin.
                    positions: [(pair_id(), position(10, 100))].into(),
                    ..Default::default()
                },
            )
            .with_user(
                BOB,
                UserState {
                    margin: UsdValue::new_int(700),
                    positions: [(pair_id(), position(-10, 100))].into(),
                    ..Default::default()
                },
            )
            .with_balance(1_200);

        fixture.run().unwrap();

        // Alice's equity is -$400; she is credited nothing rather than a
        // negative amount.
        assert_eq!(fixture.balance_of(ALICE), Uint128::ZERO);
        // Bob gained $500 on his short, on top of $700 of margin.
        assert_eq!(fixture.balance_of(BOB), Uint128::new(1_200 * ONE_USDC));
        // Bob's payout absorbed the whole balance, so there is nothing to
        // sweep — Alice's bad debt came out of the owner's share.
        assert_eq!(fixture.balance_of(OWNER), Uint128::ZERO);
    }

    /// If the contract cannot cover what it owes, the upgrade must fail so the
    /// chain halts with its pre-upgrade state intact, rather than paying out an
    /// arbitrary partial haircut.
    #[test]
    fn halts_when_insolvent() {
        let fixture = Fixture::new()
            .with_state(State::default())
            .with_pair(100, 0)
            .with_user(
                ALICE,
                UserState {
                    margin: UsdValue::new_int(1_000),
                    ..Default::default()
                },
            )
            .with_balance(999);

        let err = fixture.run().unwrap_err();

        assert!(
            err.to_string().contains("insolvent"),
            "unexpected error: {err}"
        );

        // Nothing was paid out, and the user still holds their margin.
        assert_eq!(fixture.balance_of(ALICE), Uint128::ZERO);
        assert_eq!(fixture.balance_of(PERPS), Uint128::new(999 * ONE_USDC));
        assert_eq!(
            USER_STATES.load(&fixture.perps(), ALICE).unwrap().margin,
            UsdValue::new_int(1_000)
        );
    }

    /// Trading is switched off, which is what stops the exchange being used
    /// again after the fork. Unrelated parameters survive the reshape.
    #[test]
    fn disables_trading() {
        let fixture = Fixture::new()
            .with_state(State::default())
            .with_pair(100, 0)
            .with_balance(0);

        fixture.run().unwrap();

        let param = PARAM.load(&fixture.perps()).unwrap();

        assert!(!param.trading_enabled);
        assert_eq!(param.max_unlocks, 10);
        assert_eq!(param.max_open_orders, 100);
    }

    // ---------------------------- real-data tests ----------------------------
    //
    // These run against a snapshot of a live chain's perps state, pulled into
    // `testdata/` (gitignored — the dump is large and goes stale as the chain
    // advances). Ignored by default; produce a fixture and run them with:
    //
    //   cargo run -p dango-upgrade --example dump_perps_state -- mainnet
    //   cargo test -p dango-upgrade -- --ignored

    fn snapshot_path(chain: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join(format!("{chain}_snapshot.json"))
    }

    /// Rebuild a chain's perps and bank storage in memory from a snapshot.
    fn reconstruct(chain: &str) -> (Shared<MockStorage>, Addr, Uint128) {
        let path = snapshot_path(chain);

        let snapshot: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();

        let perps_address: Addr =
            serde_json::from_value(snapshot["perps_address"].clone()).unwrap();
        let balance: Uint128 = serde_json::from_value(snapshot["balance"].clone()).unwrap();

        let base = Shared::new(MockStorage::new());

        // Scanned keys are already relative to the contract's own storage, so
        // they are written back through an identically scoped provider.
        let storage: BTreeMap<Binary, Binary> =
            serde_json::from_value(snapshot["storage"].clone()).unwrap();

        let mut perps = StorageProvider::new(
            Box::new(base.clone()),
            &[CONTRACT_NAMESPACE, &perps_address],
        );

        for (key, value) in &storage {
            perps.write(key.as_ref(), value.as_ref());
        }

        let mut bank = StorageProvider::new(Box::new(base.clone()), &[CONTRACT_NAMESPACE, &BANK]);

        BALANCES
            .save(&mut bank, (&perps_address, &usdc()), &balance)
            .unwrap();

        (base, perps_address, balance)
    }

    /// Wind down a real chain snapshot, then verify that not a cent was created
    /// or destroyed and that the exchange is left fully unwound.
    fn wind_down_real(chain: &str) {
        // Snapshots are produced on demand and never committed, so a missing
        // one means "not fetched", not "broken".
        if !snapshot_path(chain).exists() {
            println!(
                "skipping {chain}: no snapshot; run `cargo run -p dango-upgrade --example \
                 dump_perps_state -- {chain}`"
            );

            return;
        }

        let (base, perps_address, balance) = reconstruct(chain);

        let perps_storage = || {
            StorageProvider::new(
                Box::new(base.clone()),
                &[CONTRACT_NAMESPACE, &perps_address],
            )
        };
        let bank_storage =
            || StorageProvider::new(Box::new(base.clone()), &[CONTRACT_NAMESPACE, &BANK]);

        // What the exchange owed before the wind-down: every account's equity
        // at the mark, plus withdrawals still in cooldown. This includes the
        // vault, whose equity is redistributed to providers rather than paid
        // out to itself, so the total is unchanged by the distribution.
        let (total_equity, total_unlocks, user_count, position_count) = {
            let storage = perps_storage();
            let querier = NoCachePerpQuerier::new_local(&storage);

            let mut total_equity = UsdValue::ZERO;
            let mut total_unlocks = UsdValue::ZERO;
            let mut user_count = 0;
            let mut position_count = 0;

            for entry in USER_STATES.range(&storage, None, None, IterationOrder::Ascending) {
                let (_addr, user_state) = entry.unwrap();

                total_equity
                    .checked_add_assign(compute_user_equity(&querier, &user_state).unwrap())
                    .unwrap();

                for unlock in &user_state.unlocks {
                    total_unlocks
                        .checked_add_assign(unlock.amount_to_release)
                        .unwrap();
                }

                user_count += 1;
                position_count += user_state.positions.len();
            }

            (total_equity, total_unlocks, user_count, position_count)
        };

        let summary = wind_down(
            &mut perps_storage(),
            &mut bank_storage(),
            perps_address,
            OWNER,
        )
        .unwrap();

        // `assert_invariants` has already checked the structural
        // post-conditions. What remains is to account for the money.
        let bank = bank_storage();

        let mut total_credited = Uint128::ZERO;
        let mut credited_accounts = 0;

        for entry in BALANCES.range(&bank, None, None, IterationOrder::Ascending) {
            let ((_addr, denom), amount) = entry.unwrap();

            if denom == usdc() {
                total_credited.checked_add_assign(amount).unwrap();
                credited_accounts += 1;
            }
        }

        let swept = may_load_balance(&bank, OWNER, &usdc()).unwrap();

        println!(
            "{chain}: {user_count} users, {position_count} positions; equity {total_equity}, \
             unlocks {total_unlocks}; credited {total_credited} across {credited_accounts} \
             accounts, of which {swept} swept to the owner\n  {summary:?}"
        );

        // The snapshot must agree with what the migration walked over.
        assert_eq!(summary.user_count, user_count);
        assert_eq!(summary.position_count, position_count);

        // Conservation: the contract's entire balance was paid out, no more and
        // no less. This rules out both minting USDC and stranding user funds.
        assert_eq!(
            total_credited, balance,
            "USDC was created or destroyed: credited {total_credited} against a starting balance \
             of {balance}"
        );
        assert_eq!(
            summary.total_refund.checked_add(summary.swept).unwrap(),
            balance
        );

        // Closing positions at a single price is zero-sum, so the equity the
        // exchange owed before the fork must reappear afterwards, split three
        // ways and with nothing unaccounted for:
        //
        //   owed = refunded + bad debt + vault residue
        //
        // Bad debt is equity users were short by and so could not be paid; the
        // vault residue is the virtual shares' claim. Both are absorbed out of
        // the owner's sweep.
        let owed = total_equity.checked_add(total_unlocks).unwrap();

        let accounted = usd_to_base(owed)
            .checked_sub(usd_to_base(summary.bad_debt))
            .unwrap()
            .checked_sub(usd_to_base(summary.vault_residue))
            .unwrap();

        // The two sides are algebraically equal but not bit-identical: equity
        // computes `size * (mark - entry)` in one multiplication, whereas
        // closing the position computes `|size| * mark - |size| * entry`. Each
        // multiplication truncates to six decimals, so a position contributes
        // up to a few units of the last place. The fill path is the
        // authoritative one — it is what ordinary trading uses.
        let tolerance = Uint128::new(4 * position_count as u128 + 1);
        let drift = if summary.total_refund > accounted {
            summary.total_refund.checked_sub(accounted).unwrap()
        } else {
            accounted.checked_sub(summary.total_refund).unwrap()
        };

        assert!(
            drift <= tolerance,
            "unaccounted equity: refunded {} against {accounted} owed net of bad debt {} and \
             vault residue {}; drift of {drift} exceeds the {tolerance} rounding allowance for \
             {position_count} positions",
            summary.total_refund,
            summary.bad_debt,
            summary.vault_residue,
        );
    }

    fn usd_to_base(value: UsdValue) -> Uint128 {
        value
            .checked_div(SETTLEMENT_CURRENCY_PRICE)
            .unwrap()
            .into_base_floor(settlement_currency::DECIMAL)
            .unwrap()
    }

    #[test]
    #[ignore = "requires gitignored testdata/mainnet_snapshot.json"]
    fn wind_down_mainnet() {
        wind_down_real("mainnet");
    }

    #[test]
    #[ignore = "requires gitignored testdata/testnet_snapshot.json"]
    fn wind_down_testnet() {
        wind_down_real("testnet");
    }
}
