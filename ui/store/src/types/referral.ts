/**
 * Referral types matching the Rust perps contract (dango/types/src/perps.rs).
 *
 * Note: Field names use camelCase because responses pass through
 * `camelCaseJsonDeserialization` which converts the on-chain snake_case.
 */

/**
 * Cumulative referral data for a user, bucketed by day.
 * Returned by the `ReferralData` query on the perps contract.
 */
export type UserReferralData = {
  /** The user's own trading volume (cumulative). */
  volume: string;
  /** Total commission shared by this user's referrer (cumulative) — i.e. rebates received. */
  commissionSharedByReferrer: string;
  /** Number of direct referees this user has. */
  refereeCount: number;
  /** Total trading volume of this user's direct referees (cumulative). */
  refereesVolume: string;
  /** Total commission distributed from this user's referees (cumulative). */
  commissionEarnedFromReferees: string;
  /** Cumulative count of daily active direct referees. */
  cumulativeDailyActiveReferees: number;
  /** Number of direct referees that have made at least one trade. */
  cumulativeGlobalActiveReferees: number;
};

/**
 * Per-referee statistics tracked from the referrer's perspective.
 * Returned (with user_index) by the `ReferrerToRefereeStats` query.
 */
export type RefereeStats = {
  registeredAt: number;
  volume: string;
  commissionEarned: string;
  lastDayActive: number;
};

/** Referee stats enriched with the referee's user index (from the tuple key). */
export type RefereeStatsWithUser = RefereeStats & { userIndex: number };

/**
 * Referrer settings: commission rate (tier-based, read-only) + share ratio (user-settable).
 * Returned by the `ReferralSettings` query on the perps contract.
 */
export type ReferrerSettings = {
  commissionRate: string;
  shareRatio: string;
};

/**
 * Volume-tiered rate schedule.
 * `tiers` is a `BTreeMap<UsdValue, Dimensionless>` serialised as `Record<string, string>`.
 */
export type RateSchedule = {
  base: string;
  tiers: Record<string, string>;
};

/**
 * Referral-related fields extracted from the perps `Param` query response.
 */
export type ReferralParams = {
  referralActive: boolean;
  minReferrerVolume: string;
  referrerCommissionRates: RateSchedule;
};

/**
 * Ordering options for the `ReferrerToRefereeStats` query.
 */
export type ReferrerStatsOrderBy = {
  order: "Ascending" | "Descending";
  limit?: number;
  index: ReferrerStatsOrderIndex;
};

export type ReferrerStatsOrderIndex =
  | { commission: { startAfter?: string } }
  | { registerAt: { startAfter?: number } }
  | { volume: { startAfter?: string } };
