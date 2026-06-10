import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { sharesToUsd } from "@left-curve/utils";
import { useCurrentPrice } from "../../../store/src/hooks/useCurrentPrice";
import { usePerpsPairParam } from "../../../store/src/hooks/usePerpsPairParam";
import { usePerpsParam } from "../../../store/src/hooks/usePerpsParam";
import { usePerpsVaultUserShares } from "../../../store/src/hooks/usePerpsVaultUserShares";
import { createQueryClientWrapper } from "./utils/query-client";

const hookMocks = vi.hoisted(() => ({
  getPerpsPairParam: vi.fn(),
  getPerpsParam: vi.fn(),
  getPerpsVaultState: vi.fn(),
  useLivePerpsTrades: vi.fn(),
  usePerpsUserState: vi.fn(),
  usePublicClient: vi.fn(),
}));

vi.mock("../../../store/src/hooks/useLivePerpsTrades.js", () => ({
  useLivePerpsTrades: hookMocks.useLivePerpsTrades,
}));

vi.mock("../../../store/src/hooks/usePerpsUserState.js", () => ({
  usePerpsUserState: hookMocks.usePerpsUserState,
}));

vi.mock("../../../store/src/hooks/usePublicClient.js", () => ({
  usePublicClient: hookMocks.usePublicClient,
}));

const perpsParam = {
  fundingRateEpoch: "3600",
  maxUserLeverage: "25",
  minOrderSize: "0.001",
};

const pairParam = {
  baseDenom: "bridge/btc",
  initialMarginRatio: "0.1",
  maintenanceMarginRatio: "0.05",
  quoteDenom: "usd",
};

const vaultState = {
  equity: "2000",
  margin: "1500",
  shareSupply: "1000",
};

describe("DEX data hooks", () => {
  beforeEach(() => {
    hookMocks.usePublicClient.mockReturnValue({
      getPerpsPairParam: hookMocks.getPerpsPairParam,
      getPerpsParam: hookMocks.getPerpsParam,
      getPerpsVaultState: hookMocks.getPerpsVaultState,
    });
    hookMocks.usePerpsUserState.mockImplementation(
      (selector: (snapshot: { userState?: { vaultShares?: string } }) => unknown) =>
        selector({
          userState: {
            vaultShares: "125",
          },
        }),
    );
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("queries global perps params through the public client and respects disabled state", async () => {
    hookMocks.getPerpsParam.mockResolvedValue(perpsParam);

    const { result } = renderHook(() => usePerpsParam(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.data).toEqual(perpsParam));
    expect(hookMocks.getPerpsParam).toHaveBeenCalledOnce();

    vi.clearAllMocks();

    renderHook(() => usePerpsParam({ enabled: false }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(hookMocks.getPerpsParam).not.toHaveBeenCalled();
  });

  it("surfaces global perps param backend failures", async () => {
    const queryError = new Error("perps params unavailable");
    hookMocks.getPerpsParam.mockRejectedValueOnce(queryError);

    const { result } = renderHook(() => usePerpsParam(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(hookMocks.getPerpsParam).toHaveBeenCalledOnce();
    expect(result.current.error).toBe(queryError);
    expect(result.current.data).toBeUndefined();
  });

  it("queries pair params only when a pair id is available and enabled", async () => {
    hookMocks.getPerpsPairParam.mockResolvedValue(pairParam);

    const { result } = renderHook(() => usePerpsPairParam({ pairId: "BTC-USD" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.data).toEqual(pairParam));
    expect(hookMocks.getPerpsPairParam).toHaveBeenCalledWith({
      pairId: "BTC-USD",
    });

    vi.clearAllMocks();

    renderHook(() => usePerpsPairParam({ pairId: "" }), {
      wrapper: createQueryClientWrapper(),
    });
    renderHook(() => usePerpsPairParam({ enabled: false, pairId: "ETH-USD" }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(hookMocks.getPerpsPairParam).not.toHaveBeenCalled();
  });

  it("keeps pair param cache entries isolated by backend pair id", async () => {
    const btcParam = {
      ...pairParam,
      baseDenom: "bridge/btc",
      initialMarginRatio: "0.1",
    };
    const ethParam = {
      ...pairParam,
      baseDenom: "bridge/eth",
      initialMarginRatio: "0.2",
    };
    hookMocks.getPerpsPairParam.mockImplementation(async ({ pairId }: { pairId: string }) =>
      pairId === "BTC-USD" ? btcParam : ethParam,
    );

    const wrapper = createQueryClientWrapper();
    const btc = renderHook(() => usePerpsPairParam({ pairId: "BTC-USD" }), { wrapper });
    const eth = renderHook(() => usePerpsPairParam({ pairId: "ETH-USD" }), { wrapper });

    await waitFor(() => {
      expect(btc.result.current.data).toEqual(btcParam);
      expect(eth.result.current.data).toEqual(ethParam);
    });

    expect(hookMocks.getPerpsPairParam).toHaveBeenCalledWith({ pairId: "BTC-USD" });
    expect(hookMocks.getPerpsPairParam).toHaveBeenCalledWith({ pairId: "ETH-USD" });
  });

  it("surfaces pair param backend failures for the requested pair", async () => {
    const queryError = new Error("pair params unavailable");
    hookMocks.getPerpsPairParam.mockRejectedValueOnce(queryError);

    const { result } = renderHook(() => usePerpsPairParam({ pairId: "BTC-USD" }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(hookMocks.getPerpsPairParam).toHaveBeenCalledWith({
      pairId: "BTC-USD",
    });
    expect(result.current.error).toBe(queryError);
    expect(result.current.data).toBeUndefined();
  });

  it("combines user vault shares with vault state to derive the account share value", async () => {
    hookMocks.getPerpsVaultState.mockResolvedValue(vaultState);

    const { result } = renderHook(
      () =>
        usePerpsVaultUserShares({
          accountAddress: "0x7661756c74757365720000000000000000000000",
          enabled: true,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.vaultState).toEqual(vaultState));

    expect(hookMocks.usePerpsUserState).toHaveBeenCalledWith(expect.any(Function), {
      accountAddress: "0x7661756c74757365720000000000000000000000",
      enabled: true,
    });
    expect(hookMocks.getPerpsVaultState).toHaveBeenCalledOnce();
    expect(result.current.userVaultShares).toBe("125");
    expect(result.current.userSharesValue).toBe(sharesToUsd("125", "2000", "1000"));
  });

  it("does not query vault state when vault share loading is disabled", () => {
    const { result } = renderHook(
      () =>
        usePerpsVaultUserShares({
          accountAddress: "0x7661756c74757365720000000000000000000000",
          enabled: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(hookMocks.usePerpsUserState).toHaveBeenCalledWith(expect.any(Function), {
      accountAddress: "0x7661756c74757365720000000000000000000000",
      enabled: false,
    });
    expect(hookMocks.getPerpsVaultState).not.toHaveBeenCalled();
    expect(result.current.userVaultShares).toBe("125");
  });

  it("keeps user vault shares available when the vault state query fails", async () => {
    const queryError = new Error("vault state unavailable");
    hookMocks.getPerpsVaultState.mockRejectedValueOnce(queryError);

    const { result } = renderHook(
      () =>
        usePerpsVaultUserShares({
          accountAddress: "0x7661756c74757365720000000000000000000000",
          enabled: true,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(hookMocks.getPerpsVaultState).toHaveBeenCalledOnce());

    expect(hookMocks.usePerpsUserState).toHaveBeenCalledWith(expect.any(Function), {
      accountAddress: "0x7661756c74757365720000000000000000000000",
      enabled: true,
    });
    expect(result.current.vaultState).toBeUndefined();
    expect(result.current.userVaultShares).toBe("125");
    expect(result.current.userSharesValue).toBe(sharesToUsd("125", "0", "0"));
  });

  it("selects current and previous price from live perps trades with stable equality", () => {
    let capturedEquality:
      | ((
          previous: { currentPrice: string | null; previousPrice: string | null },
          next: {
            currentPrice: string | null;
            previousPrice: string | null;
          },
        ) => boolean)
      | undefined;

    hookMocks.useLivePerpsTrades.mockImplementation((selector, parameters, equalityFn) => {
      capturedEquality = equalityFn;
      expect(parameters).toEqual({
        enabled: true,
        perpsPairId: "BTC-USD",
      });
      return selector({
        currentPrice: "101",
        error: null,
        previousPrice: "100",
        status: "ready",
        trades: [],
      });
    });

    const { result } = renderHook(() =>
      useCurrentPrice({
        enabled: true,
        perpsPairId: "BTC-USD",
      }),
    );

    expect(result.current).toEqual({
      currentPrice: "101",
      previousPrice: "100",
    });
    expect(
      capturedEquality?.(
        { currentPrice: "101", previousPrice: "100" },
        { currentPrice: "101", previousPrice: "100" },
      ),
    ).toBe(true);
    expect(
      capturedEquality?.(
        { currentPrice: "101", previousPrice: "100" },
        { currentPrice: "102", previousPrice: "101" },
      ),
    ).toBe(false);
  });

  it("forwards disabled current-price lookups to the live trade stream without deriving prices", () => {
    hookMocks.useLivePerpsTrades.mockImplementation((selector, parameters, equalityFn) => {
      expect(parameters).toEqual({
        enabled: false,
        perpsPairId: undefined,
      });
      expect(
        equalityFn?.(
          { currentPrice: null, previousPrice: null },
          {
            currentPrice: null,
            previousPrice: null,
          },
        ),
      ).toBe(true);

      return selector({
        currentPrice: null,
        error: null,
        previousPrice: null,
        status: "idle",
        trades: [],
      });
    });

    const { result } = renderHook(() =>
      useCurrentPrice({
        enabled: false,
        perpsPairId: undefined,
      }),
    );

    expect(result.current).toEqual({
      currentPrice: null,
      previousPrice: null,
    });
    expect(hookMocks.useLivePerpsTrades).toHaveBeenCalledOnce();
  });
});
