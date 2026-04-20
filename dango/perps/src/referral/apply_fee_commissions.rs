use {
    crate::{
        account_factory,
        referral::{commission::calculate_commission_rate, load_referral_data},
        state::{
            FEE_SHARE_RATIO, REFEREE_TO_REFERRER, REFERRER_TO_REFEREE_STATISTICS,
            USER_REFERRAL_DATA, USER_STATES,
        },
        trade::FeeBreakdown,
        volume::round_to_day,
    },
    dango_types::{
        UsdValue,
        account_factory::UserIndex,
        perps::{FeeDistributed, Param, Referee, Referrer, ReferrerSettings, UserState},
    },
    grug::{Addr, EventBuilder, QuerierWrapper, StdResult, Storage, Timestamp},
    std::collections::BTreeMap,
};

/// Maximum number of referral chain levels to walk when calculating fee
/// commissions.
const MAX_REFERRAL_CHAIN_DEPTH: usize = 5;

/// Owned outcome of an `apply_fee_commissions` call. The map carries
/// every user state that was credited a commission (or had its margin
/// debited as the payer / vault), ready for the caller to persist.
#[derive(Debug)]
pub struct FeeCommissionsOutcome {
    pub user_states: BTreeMap<Addr, UserState>,
}

/// Calculate and apply fee commissions for all fee-paying users based on the
/// referral chain.
///
/// Commissions are funded from the post-protocol-cut fee.
/// The protocol treasury is unaffected.
///
/// **Level 1 (direct referrer):**
///  - Referee (payer) gets `vault_fee × commission_rate × share_ratio`.
///  - Referrer gets `vault_fee × commission_rate × (1 − share_ratio)`.
///
/// **Levels 2–5 (upstream referrers):**
///  - Each referrer receives the *marginal* increase in commission rate:
///    `vault_fee × (their_cr − max_cr_so_far)` — only when their rate exceeds
///    all previous referrers in the chain.
///
/// All individual commissions are credited to the corresponding user's margin
/// and the total is deducted from the vault (contract) margin.
pub fn apply_fee_commissions(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    perps_contract: Addr,
    current_time: Timestamp,
    param: &Param,
    user_states: &BTreeMap<Addr, UserState>,
    fee_breakdowns: BTreeMap<Addr, FeeBreakdown>,
    volumes: &BTreeMap<Addr, UsdValue>,
    events: &mut EventBuilder,
) -> StdResult<FeeCommissionsOutcome> {
    // Clone at entry and mutate the local copy. On `Err` the clone is
    // dropped with the rest of the call frame; the caller's
    // `&BTreeMap<Addr, UserState>` is never touched.
    let mut user_states = user_states.clone();

    if !param.referral_active {
        return Ok(FeeCommissionsOutcome { user_states });
    }

    let mut total_vault_deduction = UsdValue::ZERO;
    let mut referrer_settings_cache = BTreeMap::<UserIndex, ReferrerSettings>::new();
    let mut addr_to_user_index_cache = BTreeMap::<Addr, Option<UserIndex>>::new();

    let account_factory = account_factory(querier);

    for (payer, fee_breakdown) in fee_breakdowns {
        let vault_fee = fee_breakdown.vault_fee;
        if vault_fee.is_zero() || payer == perps_contract {
            continue;
        }

        // Resolve payer address → UserIndex.
        let Some(payer_index) = super::retrieve_user_index(
            querier,
            payer,
            account_factory,
            &mut addr_to_user_index_cache,
        ) else {
            continue;
        };

        // Look up the first referrer.
        let Some(first_referrer) = REFEREE_TO_REFERRER.may_load(storage, payer_index)? else {
            continue;
        };

        // Get the first referrer's settings.
        let first_settings = compute_referrer_settings(
            storage,
            first_referrer,
            current_time,
            param,
            &mut referrer_settings_cache,
        )?;

        let first_cr = first_settings.commission_rate;
        let first_sr = first_settings.share_ratio;

        // Referee (payer) gets: vault_fee × commission_rate × share_ratio.
        let referee_share = vault_fee.checked_mul(first_cr)?.checked_mul(first_sr)?;

        // First referrer gets: vault_fee × commission_rate × (1 − share_ratio).
        let referrer_commission = vault_fee
            .checked_mul(first_cr)?
            .checked_sub(referee_share)?;

        // Track commissions per chain level for the event.
        let mut commissions = vec![referee_share, referrer_commission];

        // Credit the referee.
        if referee_share.is_non_zero() {
            credit_commission(storage, &mut user_states, payer, referee_share)?;
            total_vault_deduction.checked_add_assign(referee_share)?;
        }

        // Credit the first referrer.
        if referrer_commission.is_non_zero()
            && let Some(referrer_addr) =
                super::retrieve_master_account(querier, first_referrer, account_factory)
        {
            credit_commission(
                storage,
                &mut user_states,
                referrer_addr,
                referrer_commission,
            )?;
            total_vault_deduction.checked_add_assign(referrer_commission)?;
        }

        // Payer's trade volume for this fill.
        let payer_volume = volumes.get(&payer).copied().unwrap_or(UsdValue::ZERO);

        // Update payer's referral data: volume + commission_shared_by_referrer.
        increment_referral_data(
            storage,
            payer_index,
            current_time,
            payer_volume,
            referee_share,
            UsdValue::ZERO,
            UsdValue::ZERO,
        )?;

        // Update first referrer's referral data: referees_volume + commission_earned_from_referees.
        increment_referral_data(
            storage,
            first_referrer,
            current_time,
            UsdValue::ZERO,
            UsdValue::ZERO,
            payer_volume,
            referrer_commission,
        )?;

        // Update per-referee statistics for the first referrer.
        update_referee_stats(
            storage,
            first_referrer,
            payer_index,
            current_time,
            payer_volume,
            referrer_commission,
        )?;

        // Walk up the referrer chain (levels 2..=MAX_REFERRAL_CHAIN_DEPTH).
        let mut current_user = first_referrer;
        let mut max_cr = first_cr;

        for _ in 1..MAX_REFERRAL_CHAIN_DEPTH {
            let Some(next_referrer) = REFEREE_TO_REFERRER.may_load(storage, current_user)? else {
                break;
            };

            let next_settings = compute_referrer_settings(
                storage,
                next_referrer,
                current_time,
                param,
                &mut referrer_settings_cache,
            )?;

            let next_cr = next_settings.commission_rate;

            // Nth referrer gets the marginal increase in commission rate.
            if next_cr > max_cr {
                let marginal = next_cr.checked_sub(max_cr)?;
                let upstream_commission = vault_fee.checked_mul(marginal)?;

                commissions.push(upstream_commission);

                if upstream_commission.is_non_zero() {
                    if let Some(addr) =
                        super::retrieve_master_account(querier, next_referrer, account_factory)
                    {
                        credit_commission(storage, &mut user_states, addr, upstream_commission)?;
                        total_vault_deduction.checked_add_assign(upstream_commission)?;
                    }

                    // Update upstream referrer's referral data: commission_earned_from_referees only.
                    increment_referral_data(
                        storage,
                        next_referrer,
                        current_time,
                        UsdValue::ZERO,
                        UsdValue::ZERO,
                        UsdValue::ZERO,
                        upstream_commission,
                    )?;
                }

                max_cr = next_cr;
            } else {
                commissions.push(UsdValue::ZERO);
            }

            current_user = next_referrer;
        }

        events.push(FeeDistributed {
            payer: payer_index,
            payer_addr: payer,
            protocol_fee: fee_breakdown.protocol_fee,
            vault_fee,
            commissions,
        })?;
    }

    // Deduct the total commission from the vault margin.
    if total_vault_deduction.is_non_zero() {
        user_states
            .get_mut(&perps_contract)
            .expect("vault must be in user_states for fee commission settlement")
            .margin
            .checked_sub_assign(total_vault_deduction)?;
    }

    Ok(FeeCommissionsOutcome { user_states })
}

/// Look up or compute referrer settings for a user, with caching.
/// N.B. This function assumes the user is a valid referrer (has set a fee share ratio).
fn compute_referrer_settings(
    storage: &dyn Storage,
    user: UserIndex,
    block_timestamp: Timestamp,
    param: &Param,
    cache: &mut BTreeMap<UserIndex, ReferrerSettings>,
) -> StdResult<ReferrerSettings> {
    if let Some(&cached) = cache.get(&user) {
        return Ok(cached);
    }

    let commission_rate = calculate_commission_rate(storage, user, block_timestamp, param)?;

    let share_ratio = FEE_SHARE_RATIO.load(storage, user)?;

    let settings = ReferrerSettings {
        commission_rate,
        share_ratio,
    };

    cache.insert(user, settings);

    Ok(settings)
}

/// Credit a fee commission to a user's margin.
///
/// If the recipient is already in `user_states`, credits directly; otherwise
/// loads from storage and inserts.
fn credit_commission(
    storage: &dyn Storage,
    user_states: &mut BTreeMap<Addr, UserState>,
    recipient: Addr,
    amount: UsdValue,
) -> StdResult<()> {
    user_states
        .entry(recipient)
        .or_insert_with(|| {
            USER_STATES
                .may_load(storage, recipient)
                .unwrap()
                .unwrap_or_default()
        })
        .margin
        .checked_add_assign(amount)?;

    Ok(())
}

/// Update cumulative referral data for a user, merging into today's bucket.
fn increment_referral_data(
    storage: &mut dyn Storage,
    user_index: UserIndex,
    current_time: Timestamp,
    volume_delta: UsdValue,
    commission_shared_delta: UsdValue,
    referees_volume_delta: UsdValue,
    commission_earned_delta: UsdValue,
) -> StdResult<()> {
    let today = round_to_day(current_time);

    let mut data = load_referral_data(storage, user_index, None)?;

    data.volume.checked_add_assign(volume_delta)?;
    data.commission_shared_by_referrer
        .checked_add_assign(commission_shared_delta)?;
    data.referees_volume
        .checked_add_assign(referees_volume_delta)?;
    data.commission_earned_from_referees
        .checked_add_assign(commission_earned_delta)?;

    USER_REFERRAL_DATA.save(storage, (user_index, today), &data)
}

/// Update per-referee statistics for a referrer.
///
/// The entry must already exist (initialized in `set_referral`).
/// Accumulates volume and commission, and increments the referrer's daily
/// active users count on the first trade of each day.
fn update_referee_stats(
    storage: &mut dyn Storage,
    referrer: Referrer,
    referee: Referee,
    current_time: Timestamp,
    volume_delta: UsdValue,
    commission_delta: UsdValue,
) -> StdResult<()> {
    let today = round_to_day(current_time);

    let mut stats = REFERRER_TO_REFEREE_STATISTICS.load(storage, (referrer, referee))?;

    stats.volume.checked_add_assign(volume_delta)?;
    stats
        .commission_earned
        .checked_add_assign(commission_delta)?;

    // If this referee hasn't traded today yet.
    if stats.last_day_active != today {
        let mut referrer_data = load_referral_data(storage, referrer, None)?;

        // Increment the daily active referees.
        referrer_data.cumulative_daily_active_referees += 1;

        // Check if this is the first trade made from the referee ever.
        // If so, increment the global active users.
        if stats.last_day_active == Timestamp::ZERO {
            referrer_data.cumulative_global_active_referees += 1;
        }

        stats.last_day_active = today;

        USER_REFERRAL_DATA.save(storage, (referrer, today), &referrer_data)?;
    }

    REFERRER_TO_REFEREE_STATISTICS.save(storage, (referrer, referee), &stats)
}
