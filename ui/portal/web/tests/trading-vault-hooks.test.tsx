import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { computeLiquidationPrice } from "../../../store/src/hooks/computeLiquidationPrice";
import { useFeeRateOverride } from "../../../store/src/hooks/useFeeRateOverride";
import { usePerpsMaxSize } from "../../../store/src/hooks/usePerpsMaxSize";
import { useVaultLiquidityState } from "../../../store/src/hooks/useVaultLiquidityState";
import { useVaultSnapshots } from "../../../store/src/hooks/useVaultSnapshots";
import { sharesToUsd, usdToShares } from "@left-curve/utils";
import { createQueryClientWrapper } from "./utils/query-client";

type SubmitTxParameters = {
  mutation: {
    invalidateKeys?: unknown[];
    mutationFn: () => Promise<unknown>;
    onSuccess?: () => void;
  };
};

const hookMocks = vi.hoisted(() => ({
  getFeeRateOverride: vi.fn(),
  getPerpsVaultState: vi.fn(),
  getVaultSnapshots: vi.fn(),
  submitTxCalls: [] as SubmitTxParameters[],
  useAccount: vi.fn(),
  useAppConfig: vi.fn(),
  useConfig: vi.fn(),
  usePerpsUserState: vi.fn(),
  usePerpsUserStateExtended: vi.fn(),
  usePublicClient: vi.fn(),
  useSigningClient: vi.fn(),
  useSubmitTx: vi.fn(),
  vaultAddLiquidity: vi.fn(),
  vaultRemoveLiquidity: vi.fn(),
}));

vi.mock("../../../store/src/hooks/useAccount.js", () => ({
  useAccount: hookMocks.useAccount,
}));

vi.mock("../../../store/src/hooks/useAppConfig.js", () => ({
  useAppConfig: hookMocks.useAppConfig,
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: hookMocks.useConfig,
}));

vi.mock("../../../store/src/hooks/usePerpsUserState.js", () => ({
  usePerpsUserState: hookMocks.usePerpsUserState,
}));

vi.mock("../../../store/src/hooks/usePerpsUserStateExtended.js", () => ({
  usePerpsUserStateExtended: hookMocks.usePerpsUserStateExtended,
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

const account = {
  address: "0x7661756c74757365720000000000000000000000",
};

const bitcoinCoin = {
  decimals: 8,
  denom: "bridge/btc",
  logoURI: "/btc.svg",
  name: "Bitcoin",
  symbol: "BTC",
  type: "native",
};

const vaultState = {
  depositWithdrawalActive: true,
  equity: "2000",
  margin: "1500",
  shareSupply: "1000",
};

describe("trading and vault hooks", () => {
  beforeEach(() => {
    hookMocks.submitTxCalls.length = 0;

    hookMocks.useAccount.mockReturnValue({
      account,
      isConnected: true,
    });
    hookMocks.useConfig.mockReturnValue({
      coins: {
        byDenom: {
          [bitcoinCoin.denom]: bitcoinCoin,
        },
      },
    });
    hookMocks.useAppConfig.mockReturnValue({
      data: {
        perpsParam: {
          vaultDepositCap: "1400",
        },
      },
    });
    hookMocks.usePublicClient.mockReturnValue({
      getFeeRateOverride: hookMocks.getFeeRateOverride,
      getPerpsVaultState: hookMocks.getPerpsVaultState,
      getVaultSnapshots: hookMocks.getVaultSnapshots,
    });
    hookMocks.useSigningClient.mockReturnValue({
      data: {
        vaultAddLiquidity: hookMocks.vaultAddLiquidity,
        vaultRemoveLiquidity: hookMocks.vaultRemoveLiquidity,
      },
    });
    hookMocks.usePerpsUserState.mockImplementation(
      (
        selector: (snapshot: {
          userState: { margin: string; unlocks: unknown[]; vaultShares: string };
        }) => unknown,
      ) =>
        selector({
          userState: {
            margin: "345",
            unlocks: [{ amount: "12", unlockTime: 123 }],
            vaultShares: "100",
          },
        }),
    );
    hookMocks.usePerpsUserStateExtended.mockImplementation(
      (selector: (snapshot: { availableMargin: string }) => unknown) =>
        selector({
          availableMargin: "250",
        }),
    );
    hookMocks.getPerpsVaultState.mockResolvedValue(vaultState);
    hookMocks.getVaultSnapshots.mockResolvedValue({
      "1700000000": {
        equity: "1000",
        shareSupply: "1000",
      },
      "1700086400": {
        equity: "1010",
        shareSupply: "1000",
      },
    });
    hookMocks.getFeeRateOverride.mockResolvedValue({
      makerFeeRate: "0.0001",
      takerFeeRate: "0.0005",
    });
    hookMocks.useSubmitTx.mockImplementation((parameters: SubmitTxParameters) => {
      hookMocks.submitTxCalls.push(parameters);

      return {
        invalidateKeys: parameters.mutation.invalidateKeys,
        isPending: false,
        mutateAsync: async () => {
          const result = await parameters.mutation.mutationFn();
          parameters.mutation.onSuccess?.();
          return result;
        },
      };
    });
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("computes max perps size across same-side, opposing, and reduce-only regimes", () => {
    const baseParameters = {
      action: "buy" as const,
      currentPositionSize: 2,
      currentPrice: 100,
      equity: 1000,
      isBaseSize: false,
      leverage: 10,
      otherPairsUsedMargin: 100,
      reduceOnly: false,
      reservedMargin: 50,
      takerFeeRate: 0.001,
    };
    const sameSide = renderHook(() => usePerpsMaxSize(baseParameters));

    expect(sameSide.result.current.availToTrade).toBe(830);
    expect(sameSide.result.current.maxSize).toBeCloseTo(8217.821782, 6);

    const opposing = renderHook(() =>
      usePerpsMaxSize({
        ...baseParameters,
        action: "sell",
        currentPositionSize: 2,
        isBaseSize: true,
      }),
    );

    expect(opposing.result.current.availToTrade).toBe(870);
    expect(opposing.result.current.maxSize).toBeCloseTo(86.138613, 5);

    const reduceOnly = renderHook(() =>
      usePerpsMaxSize({
        action: "sell",
        currentPositionSize: 2,
        currentPrice: 100,
        equity: 1000,
        isBaseSize: true,
        leverage: 10,
        otherPairsUsedMargin: 100,
        reduceOnly: true,
        reservedMargin: 50,
        takerFeeRate: 0.001,
      }),
    );

    expect(reduceOnly.result.current).toEqual({
      availToTrade: 870,
      maxSize: 2,
    });

    const invalid = renderHook(() =>
      usePerpsMaxSize({
        action: "buy",
        currentPositionSize: 0,
        currentPrice: 0,
        equity: 1000,
        isBaseSize: false,
        leverage: 10,
        otherPairsUsedMargin: 0,
        reduceOnly: false,
        reservedMargin: 0,
        takerFeeRate: 0,
      }),
    );

    expect(invalid.result.current).toEqual({
      availToTrade: 0,
      maxSize: 0,
    });
  });

  it("computes cross-margin liquidation price using other positions, maintenance margin, and funding", () => {
    const liquidationPrice = computeLiquidationPrice({
      entryPrice: 100,
      extendedPositions: {
        "perp/btcusd": {
          size: "2",
          unrealizedFunding: "10",
        },
        "perp/ethusd": {
          entryPrice: "50",
          size: "-1",
          unrealizedFunding: "2",
        },
      },
      margin: 100,
      mmr: 0.05,
      pairParams: {
        "perp/ethusd": {
          maintenanceMarginRatio: "0.1",
        },
      },
      pairPrices: {
        "perp/ethusd": {
          currentPrice: "40",
        },
      },
      size: 2,
      targetPairId: "perp/btcusd",
    });

    expect(liquidationPrice).toBeCloseTo(55.789473, 5);
    expect(
      computeLiquidationPrice({
        entryPrice: 100,
        extendedPositions: {},
        margin: 0,
        mmr: 0.05,
        pairParams: {},
        pairPrices: {},
        size: 2,
        targetPairId: "perp/btcusd",
      }),
    ).toBeNull();
  });

  it("derives vault liquidity state and submits deposit and withdrawal transactions", async () => {
    const controllers = {
      inputs: {
        depositAmount: {
          value: "12.3456789",
        },
        withdrawShares: {
          value: "25.9",
        },
      },
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useVaultLiquidityState({
          action: "deposit",
          apyWindowDays: 14,
          controllers,
          onChangeAction: vi.fn(),
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.vaultState).toEqual(vaultState));

    expect(result.current).toMatchObject({
      action: "deposit",
      isPaused: false,
      isTvlCapReached: true,
      sharePrice: "2",
      userHasShares: true,
      userMargin: "250",
      userUnlocks: [{ amount: "12", unlockTime: 123 }],
      userVaultShares: "100",
    });
    expect(result.current.sharesToReceive).toBe(usdToShares("12.345679", "2000", "1000"));
    expect(result.current.usdToReceive).toBe(sharesToUsd("26", "2000", "1000"));
    expect(result.current.userSharesValue).toBe(sharesToUsd("100", "2000", "1000"));
    expect(result.current.vaultApy).toBeDefined();

    await act(async () => {
      await result.current.deposit.mutateAsync();
    });

    expect(hookMocks.vaultAddLiquidity).toHaveBeenCalledWith({
      amount: "12.345679",
      sender: account.address,
    });
    expect(controllers.reset).toHaveBeenCalledTimes(1);
    expect(controllers.setValue).toHaveBeenCalledWith("depositAmount", "");
    expect(result.current.deposit.invalidateKeys).toEqual([["vaultState"]]);

    await act(async () => {
      await result.current.withdraw.mutateAsync();
    });

    expect(hookMocks.vaultRemoveLiquidity).toHaveBeenCalledWith({
      sender: account.address,
      sharesToBurn: "26",
    });
    expect(controllers.reset).toHaveBeenCalledTimes(2);
    expect(controllers.setValue).toHaveBeenCalledWith("withdrawShares", "");
    expect(result.current.withdraw.invalidateKeys).toEqual([["vaultState"]]);
  });

  it("keeps vault liquidity form values when backend submissions fail", async () => {
    const depositError = new Error("vault deposit failed");
    const withdrawError = new Error("vault withdraw failed");
    hookMocks.vaultAddLiquidity.mockRejectedValueOnce(depositError);
    hookMocks.vaultRemoveLiquidity.mockRejectedValueOnce(withdrawError);
    const controllers = {
      inputs: {
        depositAmount: {
          value: "12.3456789",
        },
        withdrawShares: {
          value: "25.9",
        },
      },
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useVaultLiquidityState({
          action: "deposit",
          apyWindowDays: 14,
          controllers,
          onChangeAction: vi.fn(),
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.vaultState).toEqual(vaultState));

    await expect(result.current.deposit.mutateAsync()).rejects.toThrow("vault deposit failed");
    expect(hookMocks.vaultAddLiquidity).toHaveBeenCalledWith({
      amount: "12.345679",
      sender: account.address,
    });

    await expect(result.current.withdraw.mutateAsync()).rejects.toThrow("vault withdraw failed");
    expect(hookMocks.vaultRemoveLiquidity).toHaveBeenCalledWith({
      sender: account.address,
      sharesToBurn: "26",
    });
    expect(controllers.reset).not.toHaveBeenCalled();
    expect(controllers.setValue).not.toHaveBeenCalled();
  });

  it("derives vault pause, cap, and APY window state from backend data", async () => {
    const now = new Date("2026-06-09T00:00:00.000Z").getTime();
    vi.spyOn(Date, "now").mockReturnValue(now);
    hookMocks.getPerpsVaultState.mockResolvedValueOnce({
      ...vaultState,
      depositWithdrawalActive: false,
      margin: "1399.999999",
    });
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useVaultLiquidityState({
          action: "withdraw",
          apyWindowDays: 30,
          controllers,
          onChangeAction: vi.fn(),
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.vaultState).toMatchObject({
        depositWithdrawalActive: false,
        margin: "1399.999999",
      }),
    );

    expect(hookMocks.getVaultSnapshots).toHaveBeenCalledWith({
      min: Math.floor(now / 1000) - 30 * 86_400,
    });
    expect(result.current).toMatchObject({
      action: "withdraw",
      isPaused: true,
      isTvlCapReached: false,
      sharePrice: "2",
    });
    expect(result.current.vaultApy).toBeDefined();
  });

  it("keeps account-specific vault liquidity state disabled without a connected account", async () => {
    hookMocks.useAccount.mockReturnValue({
      account: null,
      isConnected: false,
    });
    const controllers = {
      inputs: {},
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useVaultLiquidityState({
          action: "deposit",
          apyWindowDays: 14,
          controllers,
          onChangeAction: vi.fn(),
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.vaultState).toEqual(vaultState));

    expect(hookMocks.usePerpsUserState).toHaveBeenCalledWith(expect.any(Function), {
      accountAddress: undefined,
      enabled: false,
    });
    expect(hookMocks.usePerpsUserStateExtended).toHaveBeenCalledWith(expect.any(Function), {
      accountAddress: undefined,
      enabled: false,
    });
    expect(result.current).toMatchObject({
      userHasShares: false,
      userMargin: "0",
      userSharesValue: "0",
      userUnlocks: [],
      userVaultShares: "0",
    });
  });

  it("does not submit vault liquidity transactions without a connected account", async () => {
    hookMocks.useAccount.mockReturnValue({
      account: null,
      isConnected: false,
    });
    const controllers = {
      inputs: {
        depositAmount: {
          value: "12.3456789",
        },
        withdrawShares: {
          value: "25.9",
        },
      },
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useVaultLiquidityState({
          action: "deposit",
          apyWindowDays: 14,
          controllers,
          onChangeAction: vi.fn(),
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.vaultState).toEqual(vaultState));

    await expect(result.current.deposit.mutateAsync()).rejects.toThrow("no account found");
    await expect(result.current.withdraw.mutateAsync()).rejects.toThrow("no account found");

    expect(hookMocks.vaultAddLiquidity).not.toHaveBeenCalled();
    expect(hookMocks.vaultRemoveLiquidity).not.toHaveBeenCalled();
    expect(controllers.reset).not.toHaveBeenCalled();
    expect(controllers.setValue).not.toHaveBeenCalled();
  });

  it("does not submit vault liquidity transactions without a signing client", async () => {
    hookMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });
    const controllers = {
      inputs: {
        depositAmount: {
          value: "12.3456789",
        },
        withdrawShares: {
          value: "25.9",
        },
      },
      reset: vi.fn(),
      setValue: vi.fn(),
    };

    const { result } = renderHook(
      () =>
        useVaultLiquidityState({
          action: "deposit",
          apyWindowDays: 14,
          controllers,
          onChangeAction: vi.fn(),
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.vaultState).toEqual(vaultState));

    await expect(result.current.deposit.mutateAsync()).rejects.toThrow(
      "signingClient not available",
    );
    await expect(result.current.withdraw.mutateAsync()).rejects.toThrow(
      "signingClient not available",
    );

    expect(hookMocks.vaultAddLiquidity).not.toHaveBeenCalled();
    expect(hookMocks.vaultRemoveLiquidity).not.toHaveBeenCalled();
    expect(controllers.reset).not.toHaveBeenCalled();
    expect(controllers.setValue).not.toHaveBeenCalled();
  });

  it("loads vault performance snapshots with the selected period window", async () => {
    vi.spyOn(Date, "now").mockReturnValue(new Date("2026-06-08T12:00:00Z").getTime());

    const { result } = renderHook(() => useVaultSnapshots({ period: "7D" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.data).toHaveLength(2));

    expect(hookMocks.getVaultSnapshots).toHaveBeenCalledWith({
      min: 1780315200,
    });
    expect(result.current.data?.[0]).toEqual({
      dailyChange: 0,
      date: "2023-11-14T22:13:20.000Z",
      sharePrice: 1,
      timestamp: 1700000000000,
    });
    expect(result.current.data?.[1]).toMatchObject({
      date: "2023-11-15T22:13:20.000Z",
      sharePrice: 1.01,
      timestamp: 1700086400000,
    });
    expect(result.current.data?.[1].dailyChange).toBeCloseTo(1, 6);
  });

  it("sorts backend vault snapshots by fractional timestamp before computing daily changes", async () => {
    hookMocks.getVaultSnapshots.mockResolvedValueOnce({
      "1700172800.5": {
        equity: "1210",
        shareSupply: "1000",
      },
      "1700000000.5": {
        equity: "1000",
        shareSupply: "1000",
      },
      "1700086400.5": {
        equity: "1100",
        shareSupply: "1000",
      },
    });

    const { result } = renderHook(() => useVaultSnapshots({ period: "14D" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.data).toHaveLength(3));

    expect(result.current.data?.map((point) => point.timestamp)).toEqual([
      1700000000500, 1700086400500, 1700172800500,
    ]);
    expect(result.current.data?.map((point) => point.sharePrice)).toEqual([1, 1.1, 1.21]);
    expect(result.current.data?.[0].dailyChange).toBe(0);
    expect(result.current.data?.[1].dailyChange).toBeCloseTo(10, 6);
    expect(result.current.data?.[2].dailyChange).toBeCloseTo(10, 6);
  });

  it("keeps zero-share backend vault snapshots finite in performance data", async () => {
    hookMocks.getVaultSnapshots.mockResolvedValueOnce({
      "1700000000": {
        equity: "1000",
        shareSupply: "0",
      },
      "1700086400": {
        equity: "1050",
        shareSupply: "1000",
      },
      "1700172800": {
        equity: "0",
        shareSupply: "1000",
      },
    });

    const { result } = renderHook(() => useVaultSnapshots({ period: "14D" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.data).toHaveLength(3));

    expect(result.current.data).toEqual([
      expect.objectContaining({
        dailyChange: 0,
        sharePrice: 0,
      }),
      expect.objectContaining({
        dailyChange: 0,
        sharePrice: 1.05,
      }),
      expect.objectContaining({
        dailyChange: -100,
        sharePrice: 0,
      }),
    ]);
  });

  it("does not query vault performance snapshots when disabled", () => {
    const { result } = renderHook(() => useVaultSnapshots({ enabled: false, period: "30D" }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(result.current.data).toBeUndefined();
    expect(hookMocks.getVaultSnapshots).not.toHaveBeenCalled();
  });

  it("queries fee-rate overrides only for a connected account", async () => {
    const connected = renderHook(() => useFeeRateOverride(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() =>
      expect(connected.result.current).toMatchObject({
        hasOverride: true,
        override: {
          makerFeeRate: "0.0001",
          takerFeeRate: "0.0005",
        },
      }),
    );
    expect(hookMocks.getFeeRateOverride).toHaveBeenCalledWith({
      user: account.address,
    });

    hookMocks.useAccount.mockReturnValue({
      account: null,
      isConnected: false,
    });
    vi.clearAllMocks();

    const disconnected = renderHook(() => useFeeRateOverride(), {
      wrapper: createQueryClientWrapper(),
    });

    expect(disconnected.result.current.hasOverride).toBe(false);
    expect(hookMocks.getFeeRateOverride).not.toHaveBeenCalled();
  });

  it("treats missing fee-rate overrides as no override and respects disabled queries", async () => {
    hookMocks.getFeeRateOverride.mockResolvedValueOnce(null);

    const missingOverride = renderHook(() => useFeeRateOverride(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() =>
      expect(missingOverride.result.current).toMatchObject({
        hasOverride: false,
        override: null,
      }),
    );
    expect(hookMocks.getFeeRateOverride).toHaveBeenCalledWith({
      user: account.address,
    });

    vi.clearAllMocks();

    const disabled = renderHook(() => useFeeRateOverride({ enabled: false }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(disabled.result.current).toMatchObject({
      hasOverride: false,
      override: undefined,
    });
    expect(hookMocks.getFeeRateOverride).not.toHaveBeenCalled();
  });

  it("surfaces fee-rate override backend failures for connected accounts", async () => {
    const queryError = new Error("fee-rate override unavailable");
    hookMocks.getFeeRateOverride.mockRejectedValueOnce(queryError);

    const { result } = renderHook(() => useFeeRateOverride(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(hookMocks.getFeeRateOverride).toHaveBeenCalledWith({
      user: account.address,
    });
    expect(result.current).toMatchObject({
      error: queryError,
      hasOverride: false,
      override: undefined,
    });
  });
});
