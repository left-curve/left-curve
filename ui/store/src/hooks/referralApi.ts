import type { PublicClient, QueryRequest, Json } from "@left-curve/dango/types";
import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type {
  UserReferralData,
  RefereeStats,
  RefereeStatsWithUser,
  ReferrerSettings,
  ReferralParams,
  ReferrerStatsOrderBy,
} from "../types/referral.js";

/**
 * Query the perps contract via wasmSmart.
 */
async function queryPerps<T>(
  client: PublicClient,
  perpsAddress: string,
  msg: Json,
): Promise<T> {
  const request = snakeCaseJsonSerialization<QueryRequest>({
    wasmSmart: {
      contract: perpsAddress,
      msg,
    },
  });

  const response = await client.queryApp({ query: request });
  const parsed = camelCaseJsonDeserialization<{ wasmSmart: T }>(response);
  return parsed.wasmSmart;
}

/**
 * Query the referrer of a user (referee).
 * Returns the referrer's user_index or null if no referrer.
 */
export async function queryReferrer(
  client: PublicClient,
  perpsAddress: string,
  userIndex: number,
): Promise<number | null> {
  return queryPerps<number | null>(client, perpsAddress, {
    referrer: { referee: userIndex },
  });
}

/**
 * Query referral data for a user.
 * Returns cumulative volume, commissions, referee counts, etc.
 */
export async function queryReferralData(
  client: PublicClient,
  perpsAddress: string,
  userIndex: number,
  since?: number,
): Promise<UserReferralData> {
  return queryPerps<UserReferralData>(client, perpsAddress, {
    referralData: { user: userIndex, since: since != null ? String(since) : undefined },
  });
}

/**
 * Query per-referee statistics for a referrer.
 * Rust returns `Vec<(Referee, RefereeStats)>` — we flatten the tuples.
 */
export async function queryRefereeStats(
  client: PublicClient,
  perpsAddress: string,
  referrerIndex: number,
  orderBy: ReferrerStatsOrderBy,
): Promise<RefereeStatsWithUser[]> {
  const raw = await queryPerps<Array<[number, RefereeStats]>>(client, perpsAddress, {
    referrerToRefereeStats: { referrer: referrerIndex, orderBy },
  });

  return raw.map(([userIndex, stats]) => ({ ...stats, userIndex }));
}

/**
 * Query referral settings for a user.
 * Returns null if the user is not a referrer.
 */
export async function queryReferralSettings(
  client: PublicClient,
  perpsAddress: string,
  userIndex: number,
): Promise<ReferrerSettings | null> {
  return queryPerps<ReferrerSettings | null>(client, perpsAddress, {
    referralSettings: { user: userIndex },
  });
}

/**
 * Query referral params from the perps Param query.
 * Extracts referral-related fields from the full Param struct.
 */
export async function queryReferralParams(
  client: PublicClient,
  perpsAddress: string,
): Promise<ReferralParams> {
  const param = await queryPerps<{
    referralActive: boolean;
    minReferrerVolume: string;
    referrerCommissionRates: ReferralParams["referrerCommissionRates"];
  }>(client, perpsAddress, {
    param: {},
  });

  return {
    referralActive: param.referralActive,
    minReferrerVolume: param.minReferrerVolume,
    referrerCommissionRates: param.referrerCommissionRates,
  };
}
