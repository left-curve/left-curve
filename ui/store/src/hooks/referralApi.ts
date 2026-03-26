import type { PublicClient, QueryRequest, Json } from "@left-curve/dango/types";
import {
  camelCaseJsonDeserialization,
  snakeCaseJsonSerialization,
} from "@left-curve/dango/encoding";

import type {
  UserReferralData,
  RefereeStats,
  ReferralSettings,
  ReferralConfig,
  RefereeStatsOrderBy,
} from "../types/referral.js";

/**
 * Query the taxman contract
 */
async function queryTaxman<T>(
  client: PublicClient,
  taxmanAddress: string,
  msg: Json,
): Promise<T> {
  const request = snakeCaseJsonSerialization<QueryRequest>({
    wasmSmart: {
      contract: taxmanAddress,
      msg,
    },
  });

  const response = await client.queryApp({ query: request });
  const parsed = camelCaseJsonDeserialization<{ wasmSmart: T }>(response);
  return parsed.wasmSmart;
}

/**
 * Query the referrer of a user
 * Returns the referrer's user_index or null if no referrer
 */
export async function queryReferrer(
  client: PublicClient,
  taxmanAddress: string,
  userIndex: number,
): Promise<number | null> {
  return queryTaxman<number | null>(client, taxmanAddress, {
    referrer: { user: userIndex },
  });
}

/**
 * Query referral data for a user (as a referrer)
 * Returns cumulative volume, commission, and active referee count
 */
export async function queryReferralData(
  client: PublicClient,
  taxmanAddress: string,
  userIndex: number,
  since?: number,
): Promise<UserReferralData> {
  return queryTaxman<UserReferralData>(client, taxmanAddress, {
    referralData: { user: userIndex, since },
  });
}

/**
 * Query list of referees for a referrer
 * Returns stats for each referee including volume and commission
 */
export async function queryRefereeStats(
  client: PublicClient,
  taxmanAddress: string,
  referrerIndex: number,
  orderBy?: RefereeStatsOrderBy,
): Promise<RefereeStats[]> {
  return queryTaxman<RefereeStats[]>(client, taxmanAddress, {
    referrerToRefereeStats: { referrer: referrerIndex, orderBy },
  });
}

/**
 * Query referral settings for a referrer
 * Returns commission rebound rate and share ratio
 */
export async function queryReferralSettings(
  client: PublicClient,
  taxmanAddress: string,
  userIndex: number,
): Promise<ReferralSettings> {
  return queryTaxman<ReferralSettings>(client, taxmanAddress, {
    referralSettings: { user: userIndex },
  });
}

/**
 * Query trading volume for a user
 * This uses the existing VolumeByUser query on taxman
 */
export async function queryUserVolume(
  client: PublicClient,
  taxmanAddress: string,
  userIndex: number,
  since?: number,
): Promise<string> {
  return queryTaxman<string>(client, taxmanAddress, {
    volumeByUser: { user: userIndex, since },
  });
}

/**
 * Query the referral config from taxman
 * Returns system-wide referral settings including tiers
 */
export async function queryReferralConfig(
  client: PublicClient,
  taxmanAddress: string,
): Promise<ReferralConfig> {
  // The referral config is part of the taxman config
  const config = await queryTaxman<{ referral?: ReferralConfig }>(client, taxmanAddress, {
    config: {},
  });

  // Return default config if referral config is not available
  if (!config.referral) {
    return {
      default_commission_rebound: "0.1",
      tiers: [
        { min_volume: "10000000", commission_rebound: "0.2" },
        { min_volume: "100000000", commission_rebound: "0.3" },
        { min_volume: "1000000000", commission_rebound: "0.4" },
      ],
      max_share_ratio: "0.5",
    };
  }

  return config.referral;
}
