import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { usePublicClient } from "./usePublicClient.js";
import { useAppConfig } from "./useAppConfig.js";

import {
  queryReferrer,
  queryReferralData,
  queryRefereeStats,
  queryReferralSettings,
  queryUserVolume,
  queryReferralConfig,
} from "./referralApi.js";

import type {
  UserReferralData,
  RefereeStats,
  ReferralSettings,
  ReferralConfig,
  RefereeStatsOrderBy,
} from "../types/referral.js";

// ─────────────────────────────────────────────────────────────────────────────
// useReferrer - Get the referrer of a user
// ─────────────────────────────────────────────────────────────────────────────

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
    queryFn: () => queryReferrer(client!, appConfig.addresses.taxman, userIndex!),
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

// ─────────────────────────────────────────────────────────────────────────────
// useReferralData - Get referral data for a user (as referrer)
// ─────────────────────────────────────────────────────────────────────────────

export type UseReferralDataParameters = {
  userIndex: number | undefined;
  since?: number;
  enabled?: boolean;
};

export function useReferralData(parameters: UseReferralDataParameters) {
  const { userIndex, since, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const { data, isLoading, isError, error } = useQuery<UserReferralData>({
    queryKey: ["referralData", userIndex, since],
    queryFn: () => queryReferralData(client!, appConfig.addresses.taxman, userIndex!, since),
    enabled: enabled && !!userIndex && !!client,
  });

  return {
    referralData: data,
    isLoading,
    isError,
    error,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// useRefereeStats - Get list of referees for a referrer
// ─────────────────────────────────────────────────────────────────────────────

export type UseRefereeStatsParameters = {
  referrerIndex: number | undefined;
  orderBy?: RefereeStatsOrderBy;
  enabled?: boolean;
};

export function useRefereeStats(parameters: UseRefereeStatsParameters) {
  const { referrerIndex, orderBy, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const { data, isLoading, isError, error } = useQuery<RefereeStats[]>({
    queryKey: ["refereeStats", referrerIndex, orderBy],
    queryFn: () => queryRefereeStats(client!, appConfig.addresses.taxman, referrerIndex!, orderBy),
    enabled: enabled && !!referrerIndex && !!client,
  });

  return {
    referees: data ?? [],
    isLoading,
    isError,
    error,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// useReferralSettings - Get referral settings for a referrer
// ─────────────────────────────────────────────────────────────────────────────

export type UseReferralSettingsParameters = {
  userIndex: number | undefined;
  enabled?: boolean;
};

export function useReferralSettings(parameters: UseReferralSettingsParameters) {
  const { userIndex, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const { data, isLoading, isError, error } = useQuery<ReferralSettings>({
    queryKey: ["referralSettings", userIndex],
    queryFn: () => queryReferralSettings(client!, appConfig.addresses.taxman, userIndex!),
    enabled: enabled && !!userIndex && !!client,
  });

  return {
    settings: data,
    isLoading,
    isError,
    error,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// useUserVolume - Get trading volume for a user
// ─────────────────────────────────────────────────────────────────────────────

export type UseUserVolumeParameters = {
  userIndex: number | undefined;
  days?: number;
  enabled?: boolean;
};

export function useUserVolume(parameters: UseUserVolumeParameters) {
  const { userIndex, days = 30, enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const since = useMemo(() => {
    if (!days) return undefined;
    const d = new Date();
    d.setDate(d.getDate() - days);
    return Math.floor(d.getTime() / 1000);
  }, [days]);

  const { data, isLoading, isError, error } = useQuery<string>({
    queryKey: ["userVolume", userIndex, since],
    queryFn: () => queryUserVolume(client!, appConfig.addresses.taxman, userIndex!, since),
    enabled: enabled && !!userIndex && !!client,
  });

  const volume = useMemo(() => {
    if (!data) return 0;
    return Number(data);
  }, [data]);

  return {
    volume,
    volumeRaw: data,
    isLoading,
    isError,
    error,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// useReferralConfig - Get system-wide referral configuration
// ─────────────────────────────────────────────────────────────────────────────

export type UseReferralConfigParameters = {
  enabled?: boolean;
};

export function useReferralConfig(parameters: UseReferralConfigParameters = {}) {
  const { enabled = true } = parameters;
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const { data, isLoading, isError, error } = useQuery<ReferralConfig>({
    queryKey: ["referralConfig"],
    queryFn: () => queryReferralConfig(client!, appConfig.addresses.taxman),
    enabled: enabled && !!client,
  });

  return {
    config: data,
    isLoading,
    isError,
    error,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility: Generate referral code and link
// ─────────────────────────────────────────────────────────────────────────────

export function getReferralCode(userIndex: number | undefined): string {
  if (!userIndex) return "";
  return userIndex.toString();
}

export function getReferralLink(userIndex: number | undefined): string {
  if (!userIndex) return "";
  const code = getReferralCode(userIndex);
  // Use current origin for the referral link
  if (typeof window !== "undefined") {
    return `${window.location.origin}?ref=${code}`;
  }
  return `?ref=${code}`;
}
