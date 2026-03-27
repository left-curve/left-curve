use {
    crate::{
        COMMISSION_RATE_OVERRIDES, FEE_SHARE_RATIO, PARAM, REFEREE_TO_REFERRER,
        REFERRER_TO_REFEREE_STATISTICS, USER_REFERRAL_DATA, USER_STATES, query::query_volume,
        volume::round_to_day,
    },
    anyhow::{bail, ensure},
    dango_types::{
        DangoQuerier, UsdValue,
        account_factory::{self, UserIndex},
        perps::{
            CommissionRate, FeeBreakdown, FeeDistributed, FeeShareRatio, Referee, RefereeStats,
            Referral, ReferralParam, Referrer, ReferrerSettings, UserReferralData, UserState,
        },
    },
    grug::{
        Addr, Bound, Duration, EventBuilder, MutableCtx, Number, NumberConst, Op,
        Order as IterationOrder, QuerierExt, QuerierWrapper, Response, Storage, Timestamp, Uint128,
    },
    std::collections::BTreeMap,
};

/// Register a referral relationship between a referrer and a referee.
///
/// Caller must be either the account factory (during registration) or an
/// account owned by the referee.
pub fn set_referral(
    ctx: MutableCtx,
    referrer: UserIndex,
    referee: UserIndex,
) -> anyhow::Result<Response> {
    // Referrer and referee must be different users.
    ensure!(referrer != referee, "a user cannot refer themselves");

    // Caller must be the account factory or an account owned by the referee.
    let account_factory = ctx.querier.query_account_factory()?;

    if ctx.sender != account_factory {
        // If not the account factory, verify the sender is the referee.
        let account = ctx.querier.query_wasm_smart(
            account_factory,
            account_factory::QueryAccountRequest {
                address: ctx.sender,
            },
        )?;

        ensure!(
            account.owner == referee,
            "caller is not the account factory or the referee"
        );
    }

    // The referrer must have a share ratio set (i.e. has opted in as a referrer).
    ensure!(
        FEE_SHARE_RATIO.has(ctx.storage, referrer),
        "referrer {referrer} has no fee share ratio set"
    );

    // The referral relationship is immutable once set.
    REFEREE_TO_REFERRER.may_update(ctx.storage, referee, |existing| {
        if existing.is_some() {
            bail!("referee {referee} already has a referrer");
        }
        Ok(referrer)
    })?;

    // Initialize per-referee statistics for the referrer.
    REFERRER_TO_REFEREE_STATISTICS.save(ctx.storage, (referrer, referee), &RefereeStats {
        registered_at: ctx.block.timestamp,
        volume: UsdValue::ZERO,
        commission_earned: UsdValue::ZERO,
        last_day_active: Duration::from_nanos(0),
    })?;

    // Increment the referrer's referee count.
    let today = round_to_day(ctx.block.timestamp);
    let mut data = load_referral_data(ctx.storage, referrer, None)?;
    data.referee_count = data.referee_count.saturating_add(1);
    USER_REFERRAL_DATA.save(ctx.storage, (referrer, today), &data)?;

    Ok(Response::new().add_event(Referral { referrer, referee })?)
}

/// Set or update the fee share ratio for the calling user (referrer).
///
/// The share ratio can only increase, never decrease, once set.
/// The caller must have traded at least `volume_to_be_referrer` in lifetime
/// perps volume.
pub fn set_fee_share_ratio(
    ctx: MutableCtx,
    share_ratio: FeeShareRatio,
) -> anyhow::Result<Response> {
    // Share ratio must not exceed the maximum.
    ensure!(
        share_ratio <= MAX_FEE_SHARE_RATIO,
        "fee share ratio cannot exceed {MAX_FEE_SHARE_RATIO}"
    );

    // Look up the caller's user index via the account factory.
    let account_factory = ctx.querier.query_account_factory()?;

    let account =
        ctx.querier
            .query_wasm_smart(account_factory, account_factory::QueryAccountRequest {
                address: ctx.sender,
            })?;

    let user_index = account.owner;

    // Users with a commission rate override bypass the volume requirement.
    // Otherwise, the caller must have enough lifetime perps volume.
    if !COMMISSION_RATE_OVERRIDES.has(ctx.storage, user_index) {
        let param = PARAM.load(ctx.storage)?;
        let volume = query_volume(ctx.storage, ctx.sender, None)?;

        ensure!(
            volume >= param.referral.volume_to_be_referrer,
            "insufficient perps volume to become a referrer (required: {}, current: {})",
            param.referral.volume_to_be_referrer,
            volume,
        );
    }

    // If already set, the new ratio must be >= the existing one.
    if let Some(existing) = FEE_SHARE_RATIO.may_load(ctx.storage, user_index)? {
        ensure!(
            share_ratio >= existing,
            "fee share ratio can only increase (current: {existing}, proposed: {share_ratio})"
        );
    }

    FEE_SHARE_RATIO.save(ctx.storage, user_index, &share_ratio)?;

    Ok(Response::new())
}

/// Set or remove a commission rate override for a user.
///
/// Only callable by the chain owner.
pub fn set_commission_rate_override(
    ctx: MutableCtx,
    user: UserIndex,
    commission_rate: Op<CommissionRate>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    match commission_rate {
        Op::Insert(rate) => COMMISSION_RATE_OVERRIDES.save(ctx.storage, user, &rate)?,
        Op::Delete => COMMISSION_RATE_OVERRIDES.remove(ctx.storage, user),
    }

    Ok(Response::new())
}

// -------------------------------- Fee commission helpers --------------------------------

/// Maximum fee share ratio a referrer can set.
const MAX_FEE_SHARE_RATIO: FeeShareRatio = FeeShareRatio::new_percent(50);

/// Number of days in the rolling window for referees volume tiers.
const COMMISSION_LOOKBACK_DAYS: u128 = 30;

/// Resolve an address to a `UserIndex` via the account factory.
///
/// Returns `None` if the query fails (e.g. address is not a known account,
/// or the querier is not configured — as in unit tests).
pub(crate) fn retrieve_user_index(
    querier: &QuerierWrapper,
    addr: Addr,
    account_factory: Addr,
    cache: &mut BTreeMap<Addr, Option<UserIndex>>,
) -> Option<UserIndex> {
    if let Some(cached) = cache.get(&addr) {
        return *cached;
    }

    let account = querier.query_wasm_smart(account_factory, account_factory::QueryAccountRequest {
        address: addr,
    });

    match account {
        Ok(account) => Some(account.owner),
        Err(_) => None,
    }
}

/// Determine the commission rate for a referrer based on their
/// direct referees' 30-day rolling trading volume against configurable tiers.
/// N.B. This function assumes the user is a valid referrer (has set a fee share ratio).
pub(crate) fn calculate_commission_rate(
    storage: &dyn Storage,
    referrer: Referrer,
    block_timestamp: Timestamp,
    referral_param: &ReferralParam,
) -> grug::StdResult<CommissionRate> {
    // If the referrer has a custom override, use it directly.
    if let Some(override_rate) = COMMISSION_RATE_OVERRIDES.may_load(storage, referrer)? {
        return Ok(override_rate);
    }

    let today = round_to_day(block_timestamp);
    let lookback_start = today.saturating_sub(grug::Duration::from_days(COMMISSION_LOOKBACK_DAYS));

    // Load the latest cumulative data for the referrer.
    let latest = load_referral_data(storage, referrer, None)?;

    // Load the cumulative data at the start of the window.
    let start = load_referral_data(storage, referrer, Some(lookback_start))?;

    // 30-day referees volume = current cumulative - start cumulative.
    let window_referees_volume = latest
        .referees_volume
        .checked_sub(start.referees_volume)
        .unwrap_or(UsdValue::ZERO);

    // Find the highest qualifying tier.
    Ok(resolve_tiered_rate(
        referral_param.commission_rate_default,
        &referral_param.commission_rates_by_volume,
        window_referees_volume,
    ))
}

/// Resolve the highest qualifying rate from a tier map.
fn resolve_tiered_rate(
    default: CommissionRate,
    tiers: &BTreeMap<UsdValue, CommissionRate>,
    volume: UsdValue,
) -> CommissionRate {
    tiers
        .range(..=volume)
        .next_back()
        .map(|(_, &rate)| rate)
        .unwrap_or(default)
}

/// Load the cumulative referral data for a user with an optional upperbound.
/// If not specified, return the latest data.
fn load_referral_data(
    storage: &dyn Storage,
    user_index: UserIndex,
    upper_bound: Option<Timestamp>,
) -> grug::StdResult<UserReferralData> {
    let upper = upper_bound.map(Bound::Inclusive);

    USER_REFERRAL_DATA
        .prefix(user_index)
        .range(storage, None, upper, IterationOrder::Descending)
        .next()
        .transpose()
        .map(|opt| opt.map(|(_, data)| data).unwrap_or_default())
}

/// Resolve the master account for a user.
///
/// Returns `None` if the query fails.
pub(crate) fn retrieve_master_account(
    querier: &QuerierWrapper,
    user: UserIndex,
    account_factory: Addr,
) -> Option<Addr> {
    let user = querier.query_wasm_smart(
        account_factory,
        account_factory::QueryUserRequest(account_factory::UserIndexOrName::Index(user)),
    );

    match user {
        Ok(user) => Some(user.master_account()),
        Err(_) => None,
    }
}

// -------------------------------- Fee commission application --------------------------------

/// Maximum number of referral chain levels to walk when calculating fee
/// commissions.
const MAX_REFERRAL_CHAIN_DEPTH: usize = 5;

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
pub(crate) fn apply_fee_commissions(
    storage: &mut dyn Storage,
    querier: &QuerierWrapper,
    perps_contract: Addr,
    current_time: Timestamp,
    referral_param: &ReferralParam,
    user_states: &mut BTreeMap<Addr, UserState>,
    fee_breakdowns: BTreeMap<Addr, FeeBreakdown>,
    volumes: &BTreeMap<Addr, UsdValue>,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    if !referral_param.active {
        return Ok(());
    }

    let mut total_vault_deduction = UsdValue::ZERO;
    let mut referrer_settings_cache = BTreeMap::<UserIndex, ReferrerSettings>::new();
    let mut addr_to_user_index_cache = BTreeMap::<Addr, Option<UserIndex>>::new();

    let account_factory = querier.query_account_factory()?;

    for (payer, fee_breakdown) in fee_breakdowns {
        let vault_fee = fee_breakdown.vault_fee;
        if vault_fee.is_zero() || payer == perps_contract {
            continue;
        }

        // Resolve payer address → UserIndex.
        let Some(payer_index) = retrieve_user_index(
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
            referral_param,
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
            credit_commission(storage, user_states, payer, referee_share)?;
            total_vault_deduction.checked_add_assign(referee_share)?;
        }

        // Credit the first referrer.
        if referrer_commission.is_non_zero()
            && let Some(referrer_addr) =
                retrieve_master_account(querier, first_referrer, account_factory)
        {
            credit_commission(storage, user_states, referrer_addr, referrer_commission)?;
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
                referral_param,
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
                        retrieve_master_account(querier, next_referrer, account_factory)
                    {
                        credit_commission(storage, user_states, addr, upstream_commission)?;
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

    Ok(())
}

/// Look up or compute referrer settings for a user, with caching.
/// N.B. This function assumes the user is a valid referrer (has set a fee share ratio).
fn compute_referrer_settings(
    storage: &dyn Storage,
    user: UserIndex,
    block_timestamp: Timestamp,
    referral_param: &ReferralParam,
    cache: &mut BTreeMap<UserIndex, ReferrerSettings>,
) -> anyhow::Result<ReferrerSettings> {
    if let Some(&cached) = cache.get(&user) {
        return Ok(cached);
    }

    let commission_rate =
        calculate_commission_rate(storage, user, block_timestamp, referral_param)?;

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
) -> anyhow::Result<()> {
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
) -> grug::StdResult<()> {
    let today = round_to_day(current_time);

    let mut data = load_referral_data(storage, user_index, None)?;

    data.volume.checked_add_assign(volume_delta)?;
    data.commission_shared_by_referrer
        .checked_add_assign(commission_shared_delta)?;
    data.referees_volume
        .checked_add_assign(referees_volume_delta)?;
    data.commission_earned_from_referees
        .checked_add_assign(commission_earned_delta)?;

    USER_REFERRAL_DATA.save(storage, (user_index, today), &data)?;

    Ok(())
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
) -> grug::StdResult<()> {
    let today = round_to_day(current_time);

    let mut stats = REFERRER_TO_REFEREE_STATISTICS.load(storage, (referrer, referee))?;

    stats.volume.checked_add_assign(volume_delta)?;
    stats
        .commission_earned
        .checked_add_assign(commission_delta)?;

    // If this referee hasn't traded today yet, increment the referrer's
    // daily active direct referees count.
    if stats.last_day_active != today {
        stats.last_day_active = today;

        let mut data = load_referral_data(storage, referrer, None)?;
        data.cumulative_active_referees += 1;
        USER_REFERRAL_DATA.save(storage, (referrer, today), &data)?;
    }

    REFERRER_TO_REFEREE_STATISTICS.save(storage, (referrer, referee), &stats)?;

    Ok(())
}
