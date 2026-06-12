import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  getReferralCode,
  getReferralLink,
  useCommissionRateOverride,
  useReferralData,
  useReferralParams,
  useReferralSettings,
  useRefereeStats,
  useReferrer,
  useSetFeeShareRatio,
  useSetReferral,
  useVolume,
} from "../../../store/src/hooks/useReferral";
import { createQueryClientWrapper } from "./utils/query-client";

type SubmitTxMutation<TVariables> = {
  invalidateKeys?: unknown[][];
  mutationFn: (variables: TVariables) => Promise<unknown>;
  onError?: (error: unknown) => void;
  onSuccess?: () => void;
};

const hookMocks = vi.hoisted(() => ({
  queryCommissionRateOverride: vi.fn(),
  queryRefereeStats: vi.fn(),
  queryReferralData: vi.fn(),
  queryReferralParams: vi.fn(),
  queryReferralSettings: vi.fn(),
  queryReferrer: vi.fn(),
  queryVolume: vi.fn(),
  setFeeShareRatio: vi.fn(),
  setReferral: vi.fn(),
  useAccount: vi.fn(),
  useAppConfig: vi.fn(),
  usePublicClient: vi.fn(),
  useSigningClient: vi.fn(),
  useSubmitTx: vi.fn(),
}));

vi.mock("../../../store/src/hooks/referralApi.js", async (importOriginal) => {
  const actual = await importOriginal<object>();

  return {
    ...actual,
    queryCommissionRateOverride: hookMocks.queryCommissionRateOverride,
    queryRefereeStats: hookMocks.queryRefereeStats,
    queryReferralData: hookMocks.queryReferralData,
    queryReferralParams: hookMocks.queryReferralParams,
    queryReferralSettings: hookMocks.queryReferralSettings,
    queryReferrer: hookMocks.queryReferrer,
    queryVolume: hookMocks.queryVolume,
  };
});

vi.mock("../../../store/src/hooks/useAccount.js", () => ({
  useAccount: hookMocks.useAccount,
}));

vi.mock("../../../store/src/hooks/useAppConfig.js", () => ({
  useAppConfig: hookMocks.useAppConfig,
}));

vi.mock("../../../store/src/hooks/usePublicClient.js", () => ({
  usePublicClient: hookMocks.usePublicClient,
}));

vi.mock("../../../store/src/hooks/useSigningClient.js", () => ({
  useSigningClient: hookMocks.useSigningClient,
}));

vi.mock("../../../store/src/hooks/useSubmitTx.js", () => ({
  useSubmitTx: hookMocks.useSubmitTx,
}));

const publicClient = {
  id: "public-client",
};

const perpsAddress = "0x7065727073000000000000000000000000000000";

function getSubmitTxMock<TVariables>(mutation: SubmitTxMutation<TVariables>) {
  return {
    invalidateKeys: mutation.invalidateKeys,
    isPending: false,
    mutateAsync: async (variables: TVariables) => {
      try {
        const result = await mutation.mutationFn(variables);
        mutation.onSuccess?.();
        return result;
      } catch (error) {
        mutation.onError?.(error);
        throw error;
      }
    },
  };
}

describe("referral hooks", () => {
  beforeEach(() => {
    hookMocks.usePublicClient.mockReturnValue(publicClient);
    hookMocks.useAppConfig.mockReturnValue({
      data: {
        addresses: {
          perps: perpsAddress,
        },
      },
    });
    hookMocks.useAccount.mockReturnValue({
      account: {
        address: "0x73656e6465720000000000000000000000000000",
      },
    });
    hookMocks.useSigningClient.mockReturnValue({
      data: {
        setFeeShareRatio: hookMocks.setFeeShareRatio,
        setReferral: hookMocks.setReferral,
      },
    });
    hookMocks.useSubmitTx.mockImplementation(
      <TVariables,>({ mutation }: { mutation: SubmitTxMutation<TVariables> }) =>
        getSubmitTxMock(mutation),
    );
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("queries referral contract state through the configured perps contract", async () => {
    hookMocks.queryReferrer.mockResolvedValue(7);
    hookMocks.queryVolume.mockResolvedValue("12345");
    hookMocks.queryReferralData.mockResolvedValue({
      commissionEarnedFromReferees: "55",
      commissionSharedByReferrer: "11",
      cumulativeDailyActiveReferees: 4,
      cumulativeGlobalActiveReferees: 9,
      refereeCount: 3,
      refereesVolume: "90000",
      volume: "120000",
    });
    hookMocks.queryRefereeStats.mockResolvedValue([
      {
        commissionEarned: "8",
        lastDayActive: 12,
        registeredAt: 4,
        userIndex: 99,
        volume: "4500",
      },
    ]);
    hookMocks.queryReferralSettings.mockResolvedValue({
      commissionRate: "0.25",
      shareRatio: "0.40",
    });
    hookMocks.queryReferralParams.mockResolvedValue({
      minReferrerVolume: "10000",
      referralActive: true,
      referrerCommissionRates: {
        base: "0.05",
        tiers: {
          "100000": "0.10",
        },
      },
    });
    hookMocks.queryCommissionRateOverride.mockResolvedValue("0.18");

    const referrer = renderHook(() => useReferrer({ userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    const volume = renderHook(
      () => useVolume({ userAddress: "0x7472616465720000000000000000000000000000", since: 171 }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    const referralData = renderHook(() => useReferralData({ userIndex: 42, since: 172 }), {
      wrapper: createQueryClientWrapper(),
    });
    const refereeStats = renderHook(() => useRefereeStats({ referrerIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    const settings = renderHook(() => useReferralSettings({ userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    const params = renderHook(() => useReferralParams(), {
      wrapper: createQueryClientWrapper(),
    });
    const override = renderHook(() => useCommissionRateOverride({ userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(referrer.result.current.referrer).toBe(7));
    await waitFor(() => expect(volume.result.current.volume).toBe("12345"));
    await waitFor(() => expect(referralData.result.current.referralData?.refereeCount).toBe(3));
    await waitFor(() => expect(refereeStats.result.current.referees).toHaveLength(1));
    await waitFor(() => expect(settings.result.current.settings?.shareRatio).toBe("0.40"));
    await waitFor(() => expect(params.result.current.referralParams?.referralActive).toBe(true));
    await waitFor(() => expect(override.result.current.override).toBe("0.18"));

    expect(referrer.result.current.hasReferrer).toBe(true);
    expect(override.result.current.hasOverride).toBe(true);
    expect(hookMocks.queryReferrer).toHaveBeenCalledWith(publicClient, perpsAddress, 42);
    expect(hookMocks.queryVolume).toHaveBeenCalledWith(
      publicClient,
      perpsAddress,
      "0x7472616465720000000000000000000000000000",
      171,
    );
    expect(hookMocks.queryReferralData).toHaveBeenCalledWith(publicClient, perpsAddress, 42, 172);
    expect(hookMocks.queryRefereeStats).toHaveBeenCalledWith(publicClient, perpsAddress, 42, {
      index: {
        volume: {},
      },
      order: "Descending",
    });
    expect(hookMocks.queryReferralSettings).toHaveBeenCalledWith(publicClient, perpsAddress, 42);
    expect(hookMocks.queryReferralParams).toHaveBeenCalledWith(publicClient, perpsAddress);
    expect(hookMocks.queryCommissionRateOverride).toHaveBeenCalledWith(
      publicClient,
      perpsAddress,
      42,
    );
  });

  it("treats zero-valued referral contract responses as present data", async () => {
    hookMocks.queryReferrer.mockResolvedValue(0);
    hookMocks.queryCommissionRateOverride.mockResolvedValue("0");

    const referrer = renderHook(() => useReferrer({ userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    const override = renderHook(() => useCommissionRateOverride({ userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(referrer.result.current.referrer).toBe(0));
    await waitFor(() => expect(override.result.current.override).toBe("0"));

    expect(referrer.result.current.hasReferrer).toBe(true);
    expect(override.result.current.hasOverride).toBe(true);
    expect(hookMocks.queryReferrer).toHaveBeenCalledWith(publicClient, perpsAddress, 42);
    expect(hookMocks.queryCommissionRateOverride).toHaveBeenCalledWith(
      publicClient,
      perpsAddress,
      42,
    );
  });

  it("passes zero-valued referral time filters through hook queries", async () => {
    hookMocks.queryVolume.mockResolvedValue("0");
    hookMocks.queryReferralData.mockResolvedValue({
      commissionEarnedFromReferees: "0",
      commissionSharedByReferrer: "0",
      cumulativeDailyActiveReferees: 0,
      cumulativeGlobalActiveReferees: 0,
      refereeCount: 0,
      refereesVolume: "0",
      volume: "0",
    });

    const volume = renderHook(
      () =>
        useVolume({
          since: 0,
          userAddress: "0x7472616465720000000000000000000000000000",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    const referralData = renderHook(() => useReferralData({ since: 0, userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(volume.result.current.volume).toBe("0"));
    await waitFor(() => expect(referralData.result.current.referralData?.volume).toBe("0"));

    expect(hookMocks.queryVolume).toHaveBeenCalledWith(
      publicClient,
      perpsAddress,
      "0x7472616465720000000000000000000000000000",
      0,
    );
    expect(hookMocks.queryReferralData).toHaveBeenCalledWith(publicClient, perpsAddress, 42, 0);
  });

  it("does not query user-scoped referral state until required identifiers are available", () => {
    const referrer = renderHook(() => useReferrer({ userIndex: undefined }), {
      wrapper: createQueryClientWrapper(),
    });
    const volume = renderHook(() => useVolume({ userAddress: undefined }), {
      wrapper: createQueryClientWrapper(),
    });
    const referralData = renderHook(() => useReferralData({ userIndex: undefined }), {
      wrapper: createQueryClientWrapper(),
    });
    const refereeStats = renderHook(() => useRefereeStats({ referrerIndex: undefined }), {
      wrapper: createQueryClientWrapper(),
    });
    const settings = renderHook(() => useReferralSettings({ userIndex: undefined }), {
      wrapper: createQueryClientWrapper(),
    });
    const override = renderHook(() => useCommissionRateOverride({ userIndex: undefined }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(referrer.result.current.hasReferrer).toBe(false);
    expect(volume.result.current.volume).toBeUndefined();
    expect(referralData.result.current.referralData).toBeUndefined();
    expect(refereeStats.result.current.referees).toEqual([]);
    expect(settings.result.current.settings).toBeUndefined();
    expect(override.result.current.hasOverride).toBe(false);
    expect(hookMocks.queryReferrer).not.toHaveBeenCalled();
    expect(hookMocks.queryVolume).not.toHaveBeenCalled();
    expect(hookMocks.queryReferralData).not.toHaveBeenCalled();
    expect(hookMocks.queryRefereeStats).not.toHaveBeenCalled();
    expect(hookMocks.queryReferralSettings).not.toHaveBeenCalled();
    expect(hookMocks.queryCommissionRateOverride).not.toHaveBeenCalled();
  });

  it("waits for the perps contract address before querying contract-scoped referral state", () => {
    hookMocks.useAppConfig.mockReturnValue({
      data: {
        addresses: {},
      },
    });

    const volume = renderHook(
      () =>
        useVolume({
          userAddress: "0x7472616465720000000000000000000000000000",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    const referralData = renderHook(() => useReferralData({ userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    const settings = renderHook(() => useReferralSettings({ userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    const override = renderHook(() => useCommissionRateOverride({ userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(volume.result.current.volume).toBeUndefined();
    expect(referralData.result.current.referralData).toBeUndefined();
    expect(settings.result.current.settings).toBeUndefined();
    expect(override.result.current.override).toBeUndefined();
    expect(hookMocks.queryVolume).not.toHaveBeenCalled();
    expect(hookMocks.queryReferralData).not.toHaveBeenCalled();
    expect(hookMocks.queryReferralSettings).not.toHaveBeenCalled();
    expect(hookMocks.queryCommissionRateOverride).not.toHaveBeenCalled();
  });

  it("respects disabled query flags even when referral identifiers are available", () => {
    renderHook(() => useReferrer({ enabled: false, userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    renderHook(
      () =>
        useVolume({
          enabled: false,
          userAddress: "0x7472616465720000000000000000000000000000",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    renderHook(() => useReferralData({ enabled: false, userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    renderHook(() => useRefereeStats({ enabled: false, referrerIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    renderHook(() => useReferralSettings({ enabled: false, userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });
    renderHook(() => useReferralParams({ enabled: false }), {
      wrapper: createQueryClientWrapper(),
    });
    renderHook(() => useCommissionRateOverride({ enabled: false, userIndex: 42 }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(hookMocks.queryReferrer).not.toHaveBeenCalled();
    expect(hookMocks.queryVolume).not.toHaveBeenCalled();
    expect(hookMocks.queryReferralData).not.toHaveBeenCalled();
    expect(hookMocks.queryRefereeStats).not.toHaveBeenCalled();
    expect(hookMocks.queryReferralSettings).not.toHaveBeenCalled();
    expect(hookMocks.queryReferralParams).not.toHaveBeenCalled();
    expect(hookMocks.queryCommissionRateOverride).not.toHaveBeenCalled();
  });

  it("respects custom referee ordering", async () => {
    const orderBy = {
      index: {
        commission: {
          startAfter: "10",
        },
      },
      limit: 25,
      order: "Ascending" as const,
    };
    hookMocks.queryRefereeStats.mockResolvedValue([]);

    const { result } = renderHook(() => useRefereeStats({ orderBy, referrerIndex: 9 }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.referees).toEqual([]));

    expect(hookMocks.queryRefereeStats).toHaveBeenCalledWith(
      publicClient,
      perpsAddress,
      9,
      orderBy,
    );
  });

  it("submits referral registration with the connected sender and invalidation contract", async () => {
    const onSuccess = vi.fn();
    const { result } = renderHook(() => useSetReferral({ onSuccess }));

    await act(async () => {
      await result.current.mutateAsync({
        referee: 42,
        referrer: 7,
      });
    });

    expect(hookMocks.setReferral).toHaveBeenCalledWith({
      referee: 42,
      referrer: 7,
      sender: "0x73656e6465720000000000000000000000000000",
    });
    expect(result.current.invalidateKeys).toEqual([["referrer"], ["referralData"]]);
    expect(onSuccess).toHaveBeenCalledOnce();
  });

  it("preserves zero-valued referral user indexes when registering referrals", async () => {
    const { result } = renderHook(() => useSetReferral());

    await act(async () => {
      await result.current.mutateAsync({
        referee: 0,
        referrer: 0,
      });
    });

    expect(hookMocks.setReferral).toHaveBeenCalledWith({
      referee: 0,
      referrer: 0,
      sender: "0x73656e6465720000000000000000000000000000",
    });
  });

  it("submits fee share ratio updates with the connected sender and invalidation contract", async () => {
    const onSuccess = vi.fn();
    const { result } = renderHook(() => useSetFeeShareRatio({ onSuccess }));

    await act(async () => {
      await result.current.mutateAsync({
        shareRatio: "0.35",
      });
    });

    expect(hookMocks.setFeeShareRatio).toHaveBeenCalledWith({
      sender: "0x73656e6465720000000000000000000000000000",
      shareRatio: "0.35",
    });
    expect(result.current.invalidateKeys).toEqual([["referralSettings"]]);
    expect(onSuccess).toHaveBeenCalledOnce();
  });

  it("surfaces rejected referral contract writes without calling success handlers", async () => {
    const referralError = new Error("referral rejected");
    const feeShareError = new Error("fee share rejected");
    const onReferralError = vi.fn();
    const onReferralSuccess = vi.fn();
    const onFeeShareError = vi.fn();
    const onFeeShareSuccess = vi.fn();
    hookMocks.setReferral.mockRejectedValueOnce(referralError);
    hookMocks.setFeeShareRatio.mockRejectedValueOnce(feeShareError);

    const setReferral = renderHook(() =>
      useSetReferral({ onError: onReferralError, onSuccess: onReferralSuccess }),
    );
    const setFeeShareRatio = renderHook(() =>
      useSetFeeShareRatio({ onError: onFeeShareError, onSuccess: onFeeShareSuccess }),
    );

    await expect(
      setReferral.result.current.mutateAsync({
        referee: 42,
        referrer: 7,
      }),
    ).rejects.toThrow("referral rejected");
    expect(hookMocks.setReferral).toHaveBeenCalledWith({
      referee: 42,
      referrer: 7,
      sender: "0x73656e6465720000000000000000000000000000",
    });
    expect(onReferralError).toHaveBeenCalledWith(referralError);
    expect(onReferralSuccess).not.toHaveBeenCalled();

    await expect(
      setFeeShareRatio.result.current.mutateAsync({
        shareRatio: "0.35",
      }),
    ).rejects.toThrow("fee share rejected");
    expect(hookMocks.setFeeShareRatio).toHaveBeenCalledWith({
      sender: "0x73656e6465720000000000000000000000000000",
      shareRatio: "0.35",
    });
    expect(onFeeShareError).toHaveBeenCalledWith(feeShareError);
    expect(onFeeShareSuccess).not.toHaveBeenCalled();
  });

  it("rejects referral mutations when wallet state is incomplete", async () => {
    hookMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    const missingClient = renderHook(() => useSetReferral());

    await expect(
      missingClient.result.current.mutateAsync({
        referee: 42,
        referrer: 7,
      }),
    ).rejects.toThrow("No signing client available");

    hookMocks.useSigningClient.mockReturnValue({
      data: {
        setFeeShareRatio: hookMocks.setFeeShareRatio,
        setReferral: hookMocks.setReferral,
      },
    });
    hookMocks.useAccount.mockReturnValue({
      account: undefined,
    });

    const missingAccount = renderHook(() => useSetFeeShareRatio());

    await expect(
      missingAccount.result.current.mutateAsync({
        shareRatio: "0.35",
      }),
    ).rejects.toThrow("No account found");
    expect(hookMocks.setReferral).not.toHaveBeenCalled();
    expect(hookMocks.setFeeShareRatio).not.toHaveBeenCalled();
  });

  it("derives referral codes and links from valid user indexes", () => {
    expect(getReferralCode(undefined)).toBe("");
    expect(getReferralCode(42)).toBe("42");
    expect(getReferralLink(undefined)).toBe("");
    expect(getReferralLink(42)).toBe(`${window.location.origin}?ref=42`);
  });
});
