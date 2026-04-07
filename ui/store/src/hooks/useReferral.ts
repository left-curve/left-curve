import { useQuery } from "@tanstack/react-query";

import { usePublicClient } from "./usePublicClient.js";
import { useAppConfig } from "./useAppConfig.js";
import { useAccount } from "./useAccount.js";
import { useSigningClient } from "./useSigningClient.js";
import { useSubmitTx } from "./useSubmitTx.js";

import {
  queryVolume,
  queryReferrer,
  queryReferralData,
  queryRefereeStats,
  queryReferralSettings,
  queryReferralParams,
} from "./referralApi.js";

import type {
  UserReferralData,
  RefereeStatsWithUser,
  ReferrerSettings,
  ReferralParams,
  ReferrerStatsOrderBy,
} from "../types/referral.js";

export type UseReferrerParameters = {
  userIndex: number | undefined;
  enabled?: boolean;
};

export function useReferrer(parameters: UseReferrerParameters) {
  const { userIndex, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const { data, isLoading, isError, error } = useQuery<number | null>({
    queryKey: ["referrer", userIndex],
    queryFn: () => queryReferrer(client!, appConfig.addresses.perps, userIndex!),
    enabled: enabled && !!userIndex && !!client,
  });

  return {
    referrer: data,
    isLoading,
    isError,
    error,
    hasReferrer: data !== null && data !== undefined,
  };
}

export type UseVolumeParameters = {
  userAddress: string | undefined;
  since?: number;
  enabled?: boolean;
};

/**
 * Hook to query a user's cumulative perps trading volume.
 * Returns lifetime volume if `since` is undefined, or volume since the given timestamp.
 *
 * This is used to determine if a user has reached the minimum volume threshold
 * to become a referrer (e.g., $10,000 lifetime volume).
 */
export function useVolume(parameters: UseVolumeParameters) {
  const { userAddress, since, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const perpsAddress = appConfig?.addresses?.perps;

  const { data, isLoading, isError, error } = useQuery<string>({
    queryKey: ["perpsVolume", userAddress, since],
    queryFn: () => queryVolume(client!, perpsAddress!, userAddress!, since),
    enabled: enabled && !!userAddress && !!client && !!perpsAddress,
  });

  return {
    volume: data,
    isLoading,
    isError,
    error,
  };
}

export type UseReferralDataParameters = {
  userIndex: number | undefined;
  since?: number;
  enabled?: boolean;
};

export function useReferralData(parameters: UseReferralDataParameters) {
  const { userIndex, since, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const perpsAddress = appConfig?.addresses?.perps;

  const { data, isLoading, isError, error } = useQuery<UserReferralData>({
    queryKey: ["referralData", userIndex, since],
    queryFn: () => queryReferralData(client!, perpsAddress!, userIndex!, since),
    enabled: enabled && !!userIndex && !!client && !!perpsAddress,
  });

  return {
    referralData: data,
    isLoading,
    isError,
    error,
  };
}

export type UseRefereeStatsParameters = {
  referrerIndex: number | undefined;
  orderBy?: ReferrerStatsOrderBy;
  enabled?: boolean;
};

const DEFAULT_ORDER_BY: ReferrerStatsOrderBy = {
  order: "descending",
  index: { volume: {} },
};

export function useRefereeStats(parameters: UseRefereeStatsParameters) {
  const { referrerIndex, orderBy = DEFAULT_ORDER_BY, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const { data, isLoading, isError, error } = useQuery<RefereeStatsWithUser[]>({
    queryKey: ["refereeStats", referrerIndex, orderBy],
    queryFn: () => queryRefereeStats(client!, appConfig.addresses.perps, referrerIndex!, orderBy),
    enabled: enabled && !!referrerIndex && !!client,
  });

  return {
    referees: data ?? [],
    isLoading,
    isError,
    error,
  };
}

export type UseReferralSettingsParameters = {
  userIndex: number | undefined;
  enabled?: boolean;
};

export function useReferralSettings(parameters: UseReferralSettingsParameters) {
  const { userIndex, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const perpsAddress = appConfig?.addresses?.perps;

  const { data, isLoading, isError, error } = useQuery<ReferrerSettings | null>({
    queryKey: ["referralSettings", userIndex],
    queryFn: () => queryReferralSettings(client!, perpsAddress!, userIndex!),
    enabled: enabled && !!userIndex && !!client && !!perpsAddress,
  });

  return {
    settings: data,
    isLoading,
    isError,
    error,
  };
}

export type UseReferralParamsParameters = {
  enabled?: boolean;
};

export function useReferralParams(parameters: UseReferralParamsParameters = {}) {
  const { enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const { data, isLoading, isError, error } = useQuery<ReferralParams>({
    queryKey: ["referralParams"],
    queryFn: () => queryReferralParams(client!, appConfig.addresses.perps),
    enabled: enabled && !!client,
  });

  return {
    referralParams: data,
    isLoading,
    isError,
    error,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// useSetReferral - Mutation to register a referral relationship
// ─────────────────────────────────────────────────────────────────────────────

export type UseSetReferralParameters = {
  onError?: (error: unknown) => void;
  onSuccess?: () => void;
};

export function useSetReferral(parameters: UseSetReferralParameters = {}) {
  const { onError, onSuccess } = parameters;
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();

  return useSubmitTx({
    mutation: {
      invalidateKeys: [["referrer"], ["referralData"]],
      mutationFn: async (variables: { referrer: number; referee: number }) => {
        if (!signingClient) throw new Error("No signing client available");
        if (!account) throw new Error("No account found");

        await signingClient.setReferral({
          sender: account.address,
          referrer: variables.referrer,
          referee: variables.referee,
        });
      },
      onError,
      onSuccess,
    },
  });
}

export type UseSetFeeShareRatioParameters = {
  onError?: (error: unknown) => void;
  onSuccess?: () => void;
};

export function useSetFeeShareRatio(parameters: UseSetFeeShareRatioParameters = {}) {
  const { onError, onSuccess } = parameters;
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();

  return useSubmitTx({
    mutation: {
      invalidateKeys: [["referralSettings"]],
      mutationFn: async (variables: { shareRatio: string }) => {
        if (!signingClient) throw new Error("No signing client available");
        if (!account) throw new Error("No account found");

        await signingClient.setFeeShareRatio({
          sender: account.address,
          shareRatio: variables.shareRatio,
        });
      },
      onError,
      onSuccess,
    },
  });
}

export function getReferralCode(userIndex: number | undefined): string {
  if (!userIndex) return "";
  return userIndex.toString();
}

export function getReferralLink(userIndex: number | undefined): string {
  if (!userIndex) return "";
  const code = getReferralCode(userIndex);
  if (typeof window !== "undefined") {
    return `${window.location.origin}?ref=${code}`;
  }
  return `?ref=${code}`;
}
