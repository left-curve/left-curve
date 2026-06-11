import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  invalidatePerpsAccountResources,
  usePerpsAccountResourceRevision,
} from "../../../store/src/hooks/perpsAccountResourceInvalidation";
import { useLivePerpsTrades } from "../../../store/src/hooks/useLivePerpsTrades";
import {
  useAllPerpsPairStats,
  usePerpsPairStatsByPairId,
} from "../../../store/src/hooks/usePerpsPairStats";
import { useOraclePrices } from "../../../store/src/hooks/useOraclePrices";
import { usePerpsLiquidityDepth } from "../../../store/src/hooks/usePerpsLiquidityDepth";
import { usePerpsOrdersByUser } from "../../../store/src/hooks/usePerpsOrdersByUser";
import { usePerpsPairState } from "../../../store/src/hooks/usePerpsPairState";
import { usePerpsState } from "../../../store/src/hooks/usePerpsState";
import { usePerpsSubmission } from "../../../store/src/hooks/usePerpsSubmission";
import { usePerpsUserState } from "../../../store/src/hooks/usePerpsUserState";
import { usePerpsUserStateExtended } from "../../../store/src/hooks/usePerpsUserStateExtended";

type PerpsSubmissionParameters = Parameters<typeof usePerpsSubmission>[0];

type SubscriptionOptions = {
  params: unknown;
  listener: (event: unknown) => void;
  onError: (error: unknown) => void;
};

type CapturedSubscription = {
  topic: string;
  options: SubscriptionOptions;
};

const hookMocks = vi.hoisted(() => ({
  getPerpsPairStats: vi.fn(),
  publicClientQueryApp: vi.fn(),
  submitPerpsOrder: vi.fn(),
  subscriptionsSubscribe: vi.fn(),
  useAccount: vi.fn(),
  useAppConfig: vi.fn(),
  useConfig: vi.fn(),
  usePublicClient: vi.fn(),
  useSigningClient: vi.fn(),
  useSubmitTx: vi.fn(),
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
  address: "0x7472616465720000000000000000000000000000",
};

const expectedUserStateExtendedRequest = {
  wasm_smart: {
    contract: "0x7065727073000000000000000000000000000000",
    msg: {
      user_state_extended: {
        include_all: true,
        include_available_margin: true,
        include_equity: true,
        include_liquidation_price: true,
        include_maintenance_margin: true,
        include_unrealized_funding: true,
        include_unrealized_pnl: true,
        user: account.address,
      },
    },
  },
};

function getDefaultSubmissionParameters(
  overrides: Partial<PerpsSubmissionParameters> = {},
): PerpsSubmissionParameters {
  return {
    action: "buy",
    controllers: {
      reset: vi.fn(),
    },
    maxSlippage: "0.0123456",
    operation: "market",
    perpsPairId: "BTC-USD",
    priceValue: "45123.987654321",
    sizeValue: "1.23456789",
    ...overrides,
  };
}

describe("perps hooks", () => {
  let capturedSubscriptions: CapturedSubscription[] = [];

  beforeEach(() => {
    capturedSubscriptions = [];

    hookMocks.useAccount.mockReturnValue({
      account,
    });
    hookMocks.useSigningClient.mockReturnValue({
      data: {
        submitPerpsOrder: hookMocks.submitPerpsOrder,
      },
    });
    hookMocks.useConfig.mockReturnValue({
      chain: {
        id: "dango-dev-1",
      },
      subscriptions: {
        subscribe: hookMocks.subscriptionsSubscribe,
      },
    });
    hookMocks.useAppConfig.mockReturnValue({
      data: {
        addresses: {
          oracle: "0x6f7261636c650000000000000000000000000000",
          perps: "0x7065727073000000000000000000000000000000",
        },
      },
    });
    hookMocks.usePublicClient.mockReturnValue({
      getPerpsPairStats: hookMocks.getPerpsPairStats,
      queryApp: hookMocks.publicClientQueryApp,
    });
    hookMocks.subscriptionsSubscribe.mockImplementation(
      (topic: string, options: SubscriptionOptions) => {
        capturedSubscriptions.push({ topic, options });
        return vi.fn();
      },
    );
    hookMocks.publicClientQueryApp.mockResolvedValue({
      wasmSmart: {
        availableMargin: "80",
        equity: "120",
        maintenanceMargin: "25",
        positions: {
          "BTC-USD": {
            size: "1",
          },
        },
      },
    });
    hookMocks.useSubmitTx.mockImplementation(
      ({
        mutation,
      }: {
        mutation: {
          mutationFn: () => Promise<unknown>;
          onSuccess?: () => void;
        };
      }) => ({
        isPending: false,
        mutateAsync: async () => {
          const result = await mutation.mutationFn();
          mutation.onSuccess?.();
          return result;
        },
      }),
    );
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  function getCapturedSubscription(topic: string) {
    const subscription = [...capturedSubscriptions]
      .reverse()
      .find((entry) => entry.topic === topic);
    if (!subscription) throw new Error(`No captured ${topic} subscription`);
    return subscription.options;
  }

  it("submits market orders with signed size, truncation, reduce-only, and child orders", async () => {
    const reset = vi.fn();
    const onSuccess = vi.fn();
    const { result } = renderHook(() =>
      usePerpsSubmission(
        getDefaultSubmissionParameters({
          controllers: {
            reset,
          },
          reduceOnly: true,
          slPrice: "42000.9876543",
          tpPrice: "51000.1234569",
          onSuccess,
        }),
      ),
    );

    await act(async () => {
      await result.current.mutateAsync();
    });

    expect(hookMocks.submitPerpsOrder).toHaveBeenCalledWith({
      kind: {
        market: {
          maxSlippage: "0.0123456",
        },
      },
      pairId: "BTC-USD",
      reduceOnly: true,
      sender: account.address,
      size: "1.234567",
      sl: {
        maxSlippage: "0.0123456",
        triggerPrice: "42000.987654",
      },
      tp: {
        maxSlippage: "0.0123456",
        triggerPrice: "51000.123456",
      },
    });
    expect(reset).toHaveBeenCalledOnce();
    expect(onSuccess).toHaveBeenCalledOnce();
  });

  it("submits limit sells with default time-in-force and without zero-priced child orders", async () => {
    const reset = vi.fn();
    const { result } = renderHook(() =>
      usePerpsSubmission(
        getDefaultSubmissionParameters({
          action: "sell",
          controllers: {
            reset,
          },
          operation: "limit",
          slPrice: "0",
          tpPrice: "0",
        }),
      ),
    );

    await act(async () => {
      await result.current.mutateAsync();
    });

    const submittedOrder = hookMocks.submitPerpsOrder.mock.calls[0][0];
    expect(submittedOrder).toEqual({
      kind: {
        limit: {
          limitPrice: "45123.987654",
          timeInForce: "GTC",
        },
      },
      pairId: "BTC-USD",
      reduceOnly: false,
      sender: account.address,
      size: "-1.234567",
    });
    expect(submittedOrder).not.toHaveProperty("sl");
    expect(submittedOrder).not.toHaveProperty("tp");
    expect(reset).toHaveBeenCalledOnce();
  });

  it("omits non-positive and non-numeric child order prices from submitted backend messages", async () => {
    const { result } = renderHook(() =>
      usePerpsSubmission(
        getDefaultSubmissionParameters({
          slPrice: "-42000.123456",
          tpPrice: "not-a-number",
        }),
      ),
    );

    await act(async () => {
      await result.current.mutateAsync();
    });

    const submittedOrder = hookMocks.submitPerpsOrder.mock.calls[0][0];
    expect(submittedOrder).toEqual({
      kind: {
        market: {
          maxSlippage: "0.0123456",
        },
      },
      pairId: "BTC-USD",
      reduceOnly: false,
      sender: account.address,
      size: "1.234567",
    });
    expect(submittedOrder).not.toHaveProperty("sl");
    expect(submittedOrder).not.toHaveProperty("tp");
  });

  it("submits limit orders with the selected time-in-force policy", async () => {
    const { result } = renderHook(() =>
      usePerpsSubmission(
        getDefaultSubmissionParameters({
          operation: "limit",
          timeInForce: "POST",
        }),
      ),
    );

    await act(async () => {
      await result.current.mutateAsync();
    });

    expect(hookMocks.submitPerpsOrder).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: {
          limit: {
            limitPrice: "45123.987654",
            timeInForce: "POST",
          },
        },
      }),
    );
  });

  it("uses the latest edited order parameters when submitting after rerender", async () => {
    const initialReset = vi.fn();
    const latestReset = vi.fn();
    const { rerender, result } = renderHook(
      (parameters: PerpsSubmissionParameters) => usePerpsSubmission(parameters),
      {
        initialProps: getDefaultSubmissionParameters({
          action: "buy",
          controllers: {
            reset: initialReset,
          },
          operation: "market",
          sizeValue: "1.5",
        }),
      },
    );

    rerender(
      getDefaultSubmissionParameters({
        action: "sell",
        controllers: {
          reset: latestReset,
        },
        operation: "limit",
        priceValue: "60123.1234569",
        reduceOnly: true,
        sizeValue: "0.87654321",
        slPrice: "59000.7654321",
        timeInForce: "IOC",
        tpPrice: "62000.9876543",
      }),
    );

    await act(async () => {
      await result.current.mutateAsync();
    });

    expect(hookMocks.submitPerpsOrder).toHaveBeenCalledOnce();
    expect(hookMocks.submitPerpsOrder).toHaveBeenCalledWith({
      kind: {
        limit: {
          limitPrice: "60123.123456",
          timeInForce: "IOC",
        },
      },
      pairId: "BTC-USD",
      reduceOnly: true,
      sender: account.address,
      size: "-0.876543",
      sl: {
        maxSlippage: "0.0123456",
        triggerPrice: "59000.765432",
      },
      tp: {
        maxSlippage: "0.0123456",
        triggerPrice: "62000.987654",
      },
    });
    expect(initialReset).not.toHaveBeenCalled();
    expect(latestReset).toHaveBeenCalledOnce();
  });

  it("fails before signing when the submission has no wallet context", async () => {
    hookMocks.useSigningClient.mockReturnValue({
      data: null,
    });
    const missingClient = renderHook(() => usePerpsSubmission(getDefaultSubmissionParameters()));

    await expect(missingClient.result.current.mutateAsync()).rejects.toThrow(
      "No signing client available",
    );

    hookMocks.useSigningClient.mockReturnValue({
      data: {
        submitPerpsOrder: hookMocks.submitPerpsOrder,
      },
    });
    hookMocks.useAccount.mockReturnValue({
      account: null,
    });
    const missingAccount = renderHook(() => usePerpsSubmission(getDefaultSubmissionParameters()));

    await expect(missingAccount.result.current.mutateAsync()).rejects.toThrow("No account found");
    expect(hookMocks.submitPerpsOrder).not.toHaveBeenCalled();
  });

  it("keeps perps order form state when the backend rejects submission", async () => {
    const reset = vi.fn();
    const onSuccess = vi.fn();
    hookMocks.submitPerpsOrder.mockRejectedValueOnce(new Error("perps order rejected"));
    const { result } = renderHook(() =>
      usePerpsSubmission(
        getDefaultSubmissionParameters({
          controllers: {
            reset,
          },
          onSuccess,
        }),
      ),
    );

    await expect(result.current.mutateAsync()).rejects.toThrow("perps order rejected");

    expect(hookMocks.submitPerpsOrder).toHaveBeenCalledWith({
      kind: {
        market: {
          maxSlippage: "0.0123456",
        },
      },
      pairId: "BTC-USD",
      reduceOnly: false,
      sender: account.address,
      size: "1.234567",
    });
    expect(reset).not.toHaveBeenCalled();
    expect(onSuccess).not.toHaveBeenCalled();
  });

  it("partitions perps account resource revisions by chain, contract, and account", async () => {
    const common = {
      chainId: "dango-dev-1",
      perpsContract: "0x70657270732d696e76616c69646174696f6e0000",
    };
    const { result, rerender } = renderHook(
      ({ accountAddress }: { accountAddress?: string }) =>
        usePerpsAccountResourceRevision({
          ...common,
          accountAddress,
        }),
      {
        initialProps: {
          accountAddress: "0x7065727073616c69636500000000000000000000",
        },
      },
    );

    expect(result.current).toBe(0);

    act(() => {
      invalidatePerpsAccountResources({
        ...common,
        accountAddress: "0x7065727073616c69636500000000000000000000",
      });
    });

    await waitFor(() => expect(result.current).toBe(1));

    act(() => {
      invalidatePerpsAccountResources({
        ...common,
        accountAddress: "0x7065727073626f62000000000000000000000000",
      });
    });

    expect(result.current).toBe(1);

    rerender({
      accountAddress: "0x7065727073626f62000000000000000000000000",
    });
    expect(result.current).toBe(1);

    rerender({
      accountAddress: undefined,
    });
    expect(result.current).toBe(0);
  });

  it("keeps live backend streams idle when disabled or missing required scope", () => {
    const perpsState = renderHook(() => usePerpsState((snapshot) => snapshot, { enabled: false }));
    const oraclePrices = renderHook(() =>
      useOraclePrices((snapshot) => snapshot, { enabled: false }),
    );
    const pairStats = renderHook(() =>
      useAllPerpsPairStats((snapshot) => snapshot, { enabled: false }),
    );
    const pairState = renderHook(() =>
      usePerpsPairState((snapshot) => snapshot, {
        perpsPairId: undefined,
      }),
    );
    const userState = renderHook(() =>
      usePerpsUserState((snapshot) => snapshot, {
        accountAddress: undefined,
      }),
    );
    const userStateExtended = renderHook(() =>
      usePerpsUserStateExtended((snapshot) => snapshot, {
        accountAddress: undefined,
      }),
    );
    const ordersByUser = renderHook(() =>
      usePerpsOrdersByUser((snapshot) => snapshot, {
        accountAddress: undefined,
      }),
    );
    const liquidityDepth = renderHook(() =>
      usePerpsLiquidityDepth((snapshot) => snapshot, {
        bucketSize: "10",
        perpsPairId: undefined,
      }),
    );
    const trades = renderHook(() =>
      useLivePerpsTrades((snapshot) => snapshot, {
        perpsPairId: undefined,
      }),
    );

    expect(perpsState.result.current).toMatchObject({ status: "idle", state: null });
    expect(oraclePrices.result.current).toMatchObject({ status: "idle", prices: {} });
    expect(pairStats.result.current).toMatchObject({
      status: "idle",
      perpsPairStats: [],
      perpsPairStatsByPairId: {},
    });
    expect(pairState.result.current).toMatchObject({
      status: "idle",
      pairId: null,
      pairState: null,
    });
    expect(userState.result.current).toMatchObject({ status: "idle", userState: null });
    expect(userStateExtended.result.current).toMatchObject({
      status: "idle",
      availableMargin: null,
      equity: null,
      maintenanceMargin: null,
      positions: {},
    });
    expect(ordersByUser.result.current).toMatchObject({ status: "idle", orders: null });
    expect(liquidityDepth.result.current).toMatchObject({
      status: "idle",
      liquidityDepth: null,
    });
    expect(trades.result.current).toMatchObject({
      status: "idle",
      trades: [],
      currentPrice: null,
      previousPrice: null,
    });
    expect(hookMocks.subscriptionsSubscribe).not.toHaveBeenCalled();
    expect(hookMocks.publicClientQueryApp).not.toHaveBeenCalled();
  });

  it("queries extended user state with the perps contract request and consumes subscription events", async () => {
    const { result } = renderHook(() =>
      usePerpsUserStateExtended((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() =>
      expect(result.current).toMatchObject({
        availableMargin: "80",
        equity: "120",
        maintenanceMargin: "25",
        status: "ready",
      }),
    );

    expect(hookMocks.publicClientQueryApp).toHaveBeenCalledWith({
      query: expectedUserStateExtendedRequest,
    });
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: {
          httpInterval: 10000,
          interval: 5,
          request: expectedUserStateExtendedRequest,
        },
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 42,
        response: {
          wasm_smart: {
            available_margin: "95",
            equity: "140",
            maintenance_margin: "31",
            positions: {
              "BTC-USD": {
                size: "2",
                unrealized_pnl: "7",
              },
            },
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        availableMargin: "95",
        equity: "140",
        lastUpdatedBlockHeight: 42,
        maintenanceMargin: "31",
        positions: {
          "BTC-USD": {
            size: "2",
            unrealizedPnl: "7",
          },
        },
        status: "ready",
      }),
    );
  });

  it("keeps subscription-backed extended user state when HTTP bootstrap resolves later", async () => {
    let resolveBootstrap!: (response: {
      wasmSmart: {
        availableMargin: string;
        equity: string;
        maintenanceMargin: string;
        positions: Record<string, unknown>;
      };
    }) => void;
    const bootstrapQuery = new Promise<Parameters<typeof resolveBootstrap>[0]>((resolve) => {
      resolveBootstrap = resolve;
    });
    hookMocks.publicClientQueryApp.mockReturnValueOnce(bootstrapQuery);

    const { result } = renderHook(() =>
      usePerpsUserStateExtended((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 88,
        response: {
          wasm_smart: {
            available_margin: "210",
            equity: "300",
            maintenance_margin: "40",
            positions: {
              "ETH-USD": {
                size: "4",
              },
            },
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        availableMargin: "210",
        equity: "300",
        lastUpdatedBlockHeight: 88,
        maintenanceMargin: "40",
        positions: {
          "ETH-USD": {
            size: "4",
          },
        },
        status: "ready",
      }),
    );

    await act(async () => {
      resolveBootstrap({
        wasmSmart: {
          availableMargin: "80",
          equity: "120",
          maintenanceMargin: "25",
          positions: {
            "BTC-USD": {
              size: "1",
            },
          },
        },
      });
      await bootstrapQuery;
    });

    expect(result.current).toMatchObject({
      availableMargin: "210",
      equity: "300",
      lastUpdatedBlockHeight: 88,
      maintenanceMargin: "40",
      positions: {
        "ETH-USD": {
          size: "4",
        },
      },
    });
  });

  it("surfaces extended user state HTTP bootstrap failures as error snapshots", async () => {
    const bootstrapError = new Error("extended user state query failed");
    hookMocks.publicClientQueryApp.mockRejectedValueOnce(bootstrapError);

    const { result } = renderHook(() =>
      usePerpsUserStateExtended((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() =>
      expect(result.current).toMatchObject({
        availableMargin: null,
        equity: null,
        error: bootstrapError,
        lastUpdatedBlockHeight: 0,
        maintenanceMargin: null,
        positions: {},
        status: "error",
      }),
    );
    expect(hookMocks.publicClientQueryApp).toHaveBeenCalledWith({
      query: expectedUserStateExtendedRequest,
    });
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: expect.objectContaining({
          request: expectedUserStateExtendedRequest,
        }),
      }),
    );
  });

  it("surfaces malformed extended user state bootstrap responses as error snapshots", async () => {
    const malformedResponse = {
      bank: {
        balance: "0",
      },
    };
    hookMocks.publicClientQueryApp.mockResolvedValueOnce(malformedResponse);

    const { result } = renderHook(() =>
      usePerpsUserStateExtended((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() => expect(result.current.status).toBe("error"));

    expect(result.current).toMatchObject({
      availableMargin: null,
      equity: null,
      lastUpdatedBlockHeight: 0,
      maintenanceMargin: null,
      positions: {},
    });
    expect(result.current.error).toBeInstanceOf(Error);
    expect((result.current.error as Error).message).toBe(
      `expecting wasm smart response, got ${JSON.stringify(malformedResponse)}`,
    );
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: expect.objectContaining({
          request: expectedUserStateExtendedRequest,
        }),
      }),
    );
  });

  it("queries standard user state with HTTP bootstrap and subscription updates", async () => {
    const expectedRequest = {
      wasm_smart: {
        contract: "0x7065727073000000000000000000000000000000",
        msg: {
          user_state: {
            user: account.address,
          },
        },
      },
    };
    hookMocks.publicClientQueryApp.mockResolvedValueOnce({
      wasmSmart: {
        margin: "100",
        positions: {},
      },
    });

    const { result } = renderHook(() =>
      usePerpsUserState((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() =>
      expect(result.current).toMatchObject({
        status: "ready",
        userState: {
          margin: "100",
          positions: {},
        },
      }),
    );

    expect(hookMocks.publicClientQueryApp).toHaveBeenCalledWith({
      query: expectedRequest,
    });
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: {
          httpInterval: 10000,
          interval: 5,
          request: expectedRequest,
        },
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 43,
        response: {
          wasm_smart: {
            margin: "125",
            positions: {
              "BTC-USD": {
                size: "1.5",
                unrealized_pnl: "4",
              },
            },
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 43,
        status: "ready",
        userState: {
          margin: "125",
          positions: {
            "BTC-USD": {
              size: "1.5",
              unrealizedPnl: "4",
            },
          },
        },
      }),
    );
  });

  it("keeps subscription-backed standard user state when HTTP bootstrap resolves later", async () => {
    let resolveBootstrap!: (response: {
      wasmSmart: {
        margin: string;
        positions: Record<string, unknown>;
      };
    }) => void;
    const bootstrapQuery = new Promise<Parameters<typeof resolveBootstrap>[0]>((resolve) => {
      resolveBootstrap = resolve;
    });
    hookMocks.publicClientQueryApp.mockReturnValueOnce(bootstrapQuery);

    const { result } = renderHook(() =>
      usePerpsUserState((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 89,
        response: {
          wasm_smart: {
            margin: "240",
            positions: {
              "ETH-USD": {
                size: "3",
              },
            },
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 89,
        status: "ready",
        userState: {
          margin: "240",
          positions: {
            "ETH-USD": {
              size: "3",
            },
          },
        },
      }),
    );

    await act(async () => {
      resolveBootstrap({
        wasmSmart: {
          margin: "100",
          positions: {
            "BTC-USD": {
              size: "1",
            },
          },
        },
      });
      await bootstrapQuery;
    });

    expect(result.current).toMatchObject({
      lastUpdatedBlockHeight: 89,
      userState: {
        margin: "240",
        positions: {
          "ETH-USD": {
            size: "3",
          },
        },
      },
    });
  });

  it("surfaces standard user state HTTP bootstrap failures as error snapshots", async () => {
    const expectedRequest = {
      wasm_smart: {
        contract: "0x7065727073000000000000000000000000000000",
        msg: {
          user_state: {
            user: account.address,
          },
        },
      },
    };
    const bootstrapError = new Error("standard user state query failed");
    hookMocks.publicClientQueryApp.mockRejectedValueOnce(bootstrapError);

    const { result } = renderHook(() =>
      usePerpsUserState((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() =>
      expect(result.current).toMatchObject({
        error: bootstrapError,
        lastUpdatedBlockHeight: 0,
        status: "error",
        userState: null,
      }),
    );
    expect(hookMocks.publicClientQueryApp).toHaveBeenCalledWith({
      query: expectedRequest,
    });
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: expect.objectContaining({
          request: expectedRequest,
        }),
      }),
    );
  });

  it("surfaces malformed standard user state bootstrap responses as error snapshots", async () => {
    const malformedResponse = {
      bank: {
        balance: "0",
      },
    };
    hookMocks.publicClientQueryApp.mockResolvedValueOnce(malformedResponse);

    const { result } = renderHook(() =>
      usePerpsUserState((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() => expect(result.current.status).toBe("error"));

    expect(result.current).toMatchObject({
      lastUpdatedBlockHeight: 0,
      userState: null,
    });
    expect(result.current.error).toBeInstanceOf(Error);
    expect((result.current.error as Error).message).toBe(
      `expecting wasm smart response, got ${JSON.stringify(malformedResponse)}`,
    );
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: expect.objectContaining({
          request: {
            wasm_smart: {
              contract: "0x7065727073000000000000000000000000000000",
              msg: {
                user_state: {
                  user: account.address,
                },
              },
            },
          },
        }),
      }),
    );
  });

  it("subscribes to global perps state with the contract state query", async () => {
    const { result } = renderHook(() => usePerpsState((snapshot) => snapshot));
    const expectedRequest = {
      wasm_smart: {
        contract: "0x7065727073000000000000000000000000000000",
        msg: {
          state: {},
        },
      },
    };

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: {
          httpInterval: 5000,
          interval: 5,
          request: expectedRequest,
        },
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 11,
        response: {
          wasm_smart: {
            next_order_id: "7",
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 11,
        state: {
          nextOrderId: "7",
        },
        status: "ready",
      }),
    );
  });

  it("accepts global perps state events from backend block height zero", async () => {
    const { result } = renderHook(() => usePerpsState((snapshot) => snapshot));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 0,
        response: {
          wasm_smart: {
            next_order_id: "0",
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 0,
        state: {
          nextOrderId: "0",
        },
        status: "ready",
      }),
    );
  });

  it("surfaces global perps state subscription failures as error snapshots", async () => {
    const { result } = renderHook(() => usePerpsState((snapshot) => snapshot));
    const streamError = new Error("perps state stream failed");

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").onError(streamError);
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        error: streamError,
        lastUpdatedBlockHeight: 0,
        state: null,
        status: "error",
      }),
    );
  });

  it("subscribes to pair state, user orders, and liquidity depth with pair-scoped requests", async () => {
    const pairState = renderHook(() =>
      usePerpsPairState((snapshot) => snapshot, {
        perpsPairId: "ETH-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenLastCalledWith(
      "queryApp",
      expect.objectContaining({
        params: {
          httpInterval: 5000,
          interval: 5,
          request: {
            wasm_smart: {
              contract: "0x7065727073000000000000000000000000000000",
              msg: {
                pair_state: {
                  pair_id: "ETH-USD",
                },
              },
            },
          },
        },
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 12,
        response: {
          wasm_smart: {
            open_interest: "18",
          },
        },
      });
    });

    await waitFor(() =>
      expect(pairState.result.current).toMatchObject({
        lastUpdatedBlockHeight: 12,
        pairId: "ETH-USD",
        pairState: {
          openInterest: "18",
        },
        status: "ready",
      }),
    );
    pairState.unmount();
    vi.clearAllMocks();
    capturedSubscriptions = [];

    const ordersByUser = renderHook(() =>
      usePerpsOrdersByUser((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenLastCalledWith(
      "queryApp",
      expect.objectContaining({
        params: {
          httpInterval: 5000,
          interval: 5,
          request: {
            wasm_smart: {
              contract: "0x7065727073000000000000000000000000000000",
              msg: {
                orders_by_user: {
                  user: account.address,
                },
              },
            },
          },
        },
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 13,
        response: {
          wasm_smart: {
            orders: [
              {
                order_id: "order-1",
              },
            ],
          },
        },
      });
    });

    await waitFor(() =>
      expect(ordersByUser.result.current).toMatchObject({
        lastUpdatedBlockHeight: 13,
        orders: {
          orders: [
            {
              orderId: "order-1",
            },
          ],
        },
        status: "ready",
      }),
    );
    ordersByUser.unmount();
    vi.clearAllMocks();
    capturedSubscriptions = [];

    const liquidityDepth = renderHook(() =>
      usePerpsLiquidityDepth((snapshot) => snapshot, {
        bucketSize: "10",
        limit: 5,
        perpsPairId: "ETH-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenLastCalledWith(
      "queryApp",
      expect.objectContaining({
        params: {
          httpInterval: 2000,
          interval: 1,
          request: {
            wasm_smart: {
              contract: "0x7065727073000000000000000000000000000000",
              msg: {
                liquidity_depth: {
                  bucket_size: "10",
                  limit: 5,
                  pair_id: "ETH-USD",
                },
              },
            },
          },
        },
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 14,
        response: {
          wasm_smart: {
            asks: [["102", "3"]],
            bids: [["100", "2"]],
          },
        },
      });
    });

    await waitFor(() =>
      expect(liquidityDepth.result.current).toMatchObject({
        lastUpdatedBlockHeight: 14,
        liquidityDepth: {
          asks: [["102", "3"]],
          bids: [["100", "2"]],
        },
        status: "ready",
      }),
    );
  });

  it("accepts pair-state subscription events from backend block height zero", async () => {
    const { result } = renderHook(() =>
      usePerpsPairState((snapshot) => snapshot, {
        perpsPairId: "BTC-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 0,
        response: {
          wasm_smart: {
            funding_rate: "0",
            long_oi: "0",
            open_interest: "0",
            short_oi: "0",
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 0,
        pairId: "BTC-USD",
        pairState: {
          fundingRate: "0",
          longOi: "0",
          openInterest: "0",
          shortOi: "0",
        },
        status: "ready",
      }),
    );
  });

  it("accepts user order subscription events from backend block height zero", async () => {
    const { result } = renderHook(() =>
      usePerpsOrdersByUser((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 0,
        response: {
          wasm_smart: {
            orders: [],
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 0,
        orders: {
          orders: [],
        },
        status: "ready",
      }),
    );
  });

  it("accepts liquidity-depth subscription events from backend block height zero", async () => {
    const { result } = renderHook(() =>
      usePerpsLiquidityDepth((snapshot) => snapshot, {
        bucketSize: "10",
        limit: 5,
        perpsPairId: "BTC-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 0,
        response: {
          wasm_smart: {
            asks: [],
            bids: [],
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 0,
        liquidityDepth: {
          asks: [],
          bids: [],
        },
        status: "ready",
      }),
    );
  });

  it("surfaces pair-scoped subscription failures without dropping latest snapshots", async () => {
    const pairState = renderHook(() =>
      usePerpsPairState((snapshot) => snapshot, {
        perpsPairId: "ETH-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 12,
        response: {
          wasm_smart: {
            open_interest: "18",
          },
        },
      });
    });

    await waitFor(() =>
      expect(pairState.result.current).toMatchObject({
        lastUpdatedBlockHeight: 12,
        pairId: "ETH-USD",
        pairState: {
          openInterest: "18",
        },
        status: "ready",
      }),
    );

    const pairStateError = new Error("pair state stream failed");

    act(() => {
      getCapturedSubscription("queryApp").onError(pairStateError);
    });

    expect(pairState.result.current).toMatchObject({
      error: pairStateError,
      lastUpdatedBlockHeight: 12,
      pairId: "ETH-USD",
      pairState: {
        openInterest: "18",
      },
      status: "error",
    });
    pairState.unmount();
    vi.clearAllMocks();
    capturedSubscriptions = [];

    const ordersByUser = renderHook(() =>
      usePerpsOrdersByUser((snapshot) => snapshot, {
        accountAddress: account.address,
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 13,
        response: {
          wasm_smart: {
            orders: [
              {
                order_id: "order-1",
              },
            ],
          },
        },
      });
    });

    await waitFor(() =>
      expect(ordersByUser.result.current).toMatchObject({
        lastUpdatedBlockHeight: 13,
        orders: {
          orders: [
            {
              orderId: "order-1",
            },
          ],
        },
        status: "ready",
      }),
    );

    const ordersError = new Error("orders stream failed");

    act(() => {
      getCapturedSubscription("queryApp").onError(ordersError);
    });

    expect(ordersByUser.result.current).toMatchObject({
      error: ordersError,
      lastUpdatedBlockHeight: 13,
      orders: {
        orders: [
          {
            orderId: "order-1",
          },
        ],
      },
      status: "error",
    });
    ordersByUser.unmount();
    vi.clearAllMocks();
    capturedSubscriptions = [];

    const liquidityDepth = renderHook(() =>
      usePerpsLiquidityDepth((snapshot) => snapshot, {
        bucketSize: "10",
        limit: 5,
        perpsPairId: "ETH-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 14,
        response: {
          wasm_smart: {
            asks: [["102", "3"]],
            bids: [["100", "2"]],
          },
        },
      });
    });

    await waitFor(() =>
      expect(liquidityDepth.result.current).toMatchObject({
        lastUpdatedBlockHeight: 14,
        liquidityDepth: {
          asks: [["102", "3"]],
          bids: [["100", "2"]],
        },
        status: "ready",
      }),
    );

    const liquidityError = new Error("liquidity depth stream failed");

    act(() => {
      getCapturedSubscription("queryApp").onError(liquidityError);
    });

    expect(liquidityDepth.result.current).toMatchObject({
      error: liquidityError,
      lastUpdatedBlockHeight: 14,
      liquidityDepth: {
        asks: [["102", "3"]],
        bids: [["100", "2"]],
      },
      status: "error",
    });
  });

  it("subscribes to oracle prices through the configured oracle contract", async () => {
    const { result } = renderHook(() => useOraclePrices((snapshot) => snapshot));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: {
          httpInterval: 2000,
          interval: 1,
          request: {
            wasm_smart: {
              contract: "0x6f7261636c650000000000000000000000000000",
              msg: {
                prices: {},
              },
            },
          },
        },
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 15,
        response: {
          wasm_smart: {
            "bridge/usdc": {
              humanized_price: "1.00",
              market_session: "open",
              timestamp: "1700000000",
            },
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 15,
        prices: {
          "bridge/usdc": {
            humanizedPrice: "1.00",
            marketSession: "open",
            timestamp: "1700000000",
          },
        },
        status: "ready",
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 14,
        response: {
          wasm_smart: {
            "bridge/usdc": {
              humanized_price: "0.99",
              market_session: "open",
              timestamp: "1699999999",
            },
          },
        },
      });
    });

    expect(result.current).toMatchObject({
      lastUpdatedBlockHeight: 15,
      prices: {
        "bridge/usdc": {
          humanizedPrice: "1.00",
          timestamp: "1700000000",
        },
      },
    });

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 16,
        response: {
          wasm_smart: {
            "bridge/usdc": {
              humanized_price: "1.00",
              market_session: "open",
              timestamp: "1700000060",
            },
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 16,
        prices: {
          "bridge/usdc": {
            humanizedPrice: "1.00",
            marketSession: "open",
            timestamp: "1700000060",
          },
        },
        status: "ready",
      }),
    );
  });

  it("accepts oracle price events from backend block height zero", async () => {
    const { result } = renderHook(() => useOraclePrices((snapshot) => snapshot));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 0,
        response: {
          wasm_smart: {
            "bridge/usdc": {
              humanized_price: "0",
              market_session: "closed",
              timestamp: "0",
            },
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 0,
        prices: {
          "bridge/usdc": {
            humanizedPrice: "0",
            marketSession: "closed",
            timestamp: "0",
          },
        },
        status: "ready",
      }),
    );
  });

  it("surfaces oracle price subscription failures without dropping latest prices", async () => {
    const { result } = renderHook(() => useOraclePrices((snapshot) => snapshot));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "queryApp",
      expect.objectContaining({
        params: expect.objectContaining({
          request: {
            wasm_smart: {
              contract: "0x6f7261636c650000000000000000000000000000",
              msg: {
                prices: {},
              },
            },
          },
        }),
      }),
    );

    act(() => {
      getCapturedSubscription("queryApp").listener({
        block_height: 15,
        response: {
          wasm_smart: {
            "bridge/usdc": {
              humanized_price: "1.00",
              market_session: "open",
              timestamp: "1700000000",
            },
          },
        },
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        lastUpdatedBlockHeight: 15,
        prices: {
          "bridge/usdc": {
            humanizedPrice: "1.00",
            timestamp: "1700000000",
          },
        },
        status: "ready",
      }),
    );

    const streamError = new Error("oracle prices stream failed");

    act(() => {
      getCapturedSubscription("queryApp").onError(streamError);
    });

    expect(result.current).toMatchObject({
      error: streamError,
      lastUpdatedBlockHeight: 15,
      prices: {
        "bridge/usdc": {
          humanizedPrice: "1.00",
          timestamp: "1700000000",
        },
      },
      status: "error",
    });
  });

  it("normalizes all pair stats from the live stats subscription", async () => {
    const { result } = renderHook(() => useAllPerpsPairStats((snapshot) => snapshot));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "allPerpsPairStats",
      expect.objectContaining({
        params: {
          httpInterval: 5000,
        },
      }),
    );

    act(() => {
      getCapturedSubscription("allPerpsPairStats").listener({
        allPerpsPairStats: [
          {
            currentPrice: "110",
            pairId: "BTC-USD",
            price24HAgo: "100",
            priceChange24H: null,
            volume24H: "2500",
          },
          {
            currentPrice: "48",
            pairId: "ETH-USD",
            price24HAgo: "50",
            priceChange24H: "-3.5",
            volume24H: "900",
          },
        ],
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        perpsPairStats: [
          {
            currentPrice: "110",
            pairId: "BTC-USD",
            price24HAgo: "100",
            priceChange24H: "10",
            volume24H: "2500",
          },
          {
            currentPrice: "48",
            pairId: "ETH-USD",
            price24HAgo: "50",
            priceChange24H: "-3.5",
            volume24H: "900",
          },
        ],
        perpsPairStatsByPairId: {
          "BTC-USD": {
            priceChange24H: "10",
          },
          "ETH-USD": {
            priceChange24H: "-3.5",
          },
        },
        status: "ready",
      }),
    );
  });

  it("normalizes malformed live pair stats without leaking invalid price changes", async () => {
    const { result } = renderHook(() => useAllPerpsPairStats((snapshot) => snapshot));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("allPerpsPairStats").listener({
        allPerpsPairStats: [
          {
            currentPrice: "75",
            pairId: "ETH-USD",
            price24HAgo: "60",
            priceChange24H: "not-a-decimal",
            volume24H: "1234.5",
          },
          {
            currentPrice: "100",
            pairId: "ZERO-USD",
            price24HAgo: "0",
            priceChange24H: null,
            volume24H: "0",
          },
        ],
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        perpsPairStats: [
          {
            currentPrice: "75",
            pairId: "ETH-USD",
            price24HAgo: "60",
            priceChange24H: "25",
            volume24H: "1234.5",
          },
          {
            currentPrice: "100",
            pairId: "ZERO-USD",
            price24HAgo: "0",
            priceChange24H: null,
            volume24H: "0",
          },
        ],
        perpsPairStatsByPairId: {
          "ETH-USD": {
            priceChange24H: "25",
          },
          "ZERO-USD": {
            priceChange24H: null,
          },
        },
        status: "ready",
      }),
    );
  });

  it("surfaces all pair stats subscription failures without dropping latest stats", async () => {
    const { result } = renderHook(() => useAllPerpsPairStats((snapshot) => snapshot));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "allPerpsPairStats",
      expect.objectContaining({
        params: {
          httpInterval: 5000,
        },
      }),
    );

    act(() => {
      getCapturedSubscription("allPerpsPairStats").listener({
        allPerpsPairStats: [
          {
            currentPrice: "110",
            pairId: "BTC-USD",
            price24HAgo: "100",
            priceChange24H: null,
            volume24H: "2500",
          },
        ],
      });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        perpsPairStats: [
          {
            currentPrice: "110",
            pairId: "BTC-USD",
            price24HAgo: "100",
            priceChange24H: "10",
            volume24H: "2500",
          },
        ],
        perpsPairStatsByPairId: {
          "BTC-USD": {
            priceChange24H: "10",
          },
        },
        status: "ready",
      }),
    );

    const streamError = new Error("pair stats stream failed");

    act(() => {
      getCapturedSubscription("allPerpsPairStats").onError(streamError);
    });

    expect(result.current).toMatchObject({
      error: streamError,
      perpsPairStats: [
        {
          currentPrice: "110",
          pairId: "BTC-USD",
          price24HAgo: "100",
          priceChange24H: "10",
          volume24H: "2500",
        },
      ],
      perpsPairStatsByPairId: {
        "BTC-USD": {
          priceChange24H: "10",
        },
      },
      status: "error",
    });
  });

  it("selects and normalizes single pair stats from the live stats subscription", async () => {
    const { result } = renderHook(() => usePerpsPairStatsByPairId({ pairId: "perp/btcusd" }));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("allPerpsPairStats").listener({
        allPerpsPairStats: [
          {
            currentPrice: "121.5",
            pairId: "perp/btcusd",
            price24HAgo: "100",
            priceChange24H: null,
            volume24H: "987654.321",
          },
        ],
      });
    });

    await waitFor(() =>
      expect(result.current).toEqual({
        currentPrice: "121.5",
        pairId: "perp/btcusd",
        price24HAgo: "100",
        priceChange24H: "21.5",
        volume24H: "987654.321",
      }),
    );
  });

  it("preserves explicit zero price-change values from live pair stats", async () => {
    const { result } = renderHook(() => usePerpsPairStatsByPairId({ pairId: "perp/btcusd" }));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("allPerpsPairStats").listener({
        allPerpsPairStats: [
          {
            currentPrice: "121.5",
            pairId: "perp/btcusd",
            price24HAgo: "100",
            priceChange24H: "0",
            volume24H: "987654.321",
          },
        ],
      });
    });

    await waitFor(() =>
      expect(result.current).toEqual({
        currentPrice: "121.5",
        pairId: "perp/btcusd",
        price24HAgo: "100",
        priceChange24H: "0",
        volume24H: "987654.321",
      }),
    );
  });

  it("falls back to computed pair stats when live price-change values are malformed", async () => {
    const { result } = renderHook(() => usePerpsPairStatsByPairId({ pairId: "perp/ethusd" }));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("allPerpsPairStats").listener({
        allPerpsPairStats: [
          {
            currentPrice: "75",
            pairId: "perp/ethusd",
            price24HAgo: "60",
            priceChange24H: "not-a-decimal",
            volume24H: "1234.5",
          },
        ],
      });
    });

    await waitFor(() =>
      expect(result.current).toEqual({
        currentPrice: "75",
        pairId: "perp/ethusd",
        price24HAgo: "60",
        priceChange24H: "25",
        volume24H: "1234.5",
      }),
    );
  });

  it("keeps selected pair stats available after live stats stream failures", async () => {
    const { result } = renderHook(() => usePerpsPairStatsByPairId({ pairId: "perp/btcusd" }));

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    act(() => {
      getCapturedSubscription("allPerpsPairStats").listener({
        allPerpsPairStats: [
          {
            currentPrice: "121.5",
            pairId: "perp/btcusd",
            price24HAgo: "100",
            priceChange24H: null,
            volume24H: "987654.321",
          },
        ],
      });
    });

    await waitFor(() => expect(result.current?.priceChange24H).toBe("21.5"));

    act(() => {
      getCapturedSubscription("allPerpsPairStats").onError(new Error("pair stats stream failed"));
    });

    expect(result.current).toEqual({
      currentPrice: "121.5",
      pairId: "perp/btcusd",
      price24HAgo: "100",
      priceChange24H: "21.5",
      volume24H: "987654.321",
    });
  });

  it("does not start single pair stats without an enabled pair id", () => {
    const disabled = renderHook(() =>
      usePerpsPairStatsByPairId({ enabled: false, pairId: "perp/btcusd" }),
    );
    const emptyPairId = renderHook(() => usePerpsPairStatsByPairId({ pairId: "" }));

    expect(hookMocks.subscriptionsSubscribe).not.toHaveBeenCalled();
    expect(disabled.result.current).toBeNull();
    expect(emptyPairId.result.current).toBeNull();
  });

  it("buffers live perps trades, ignores maker echoes, and tracks price direction", async () => {
    const { result } = renderHook(() =>
      useLivePerpsTrades((snapshot) => snapshot, {
        perpsPairId: "BTC-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "perpsTrades",
      expect.objectContaining({
        params: {
          pairId: "BTC-USD",
        },
      }),
    );

    vi.useFakeTimers();
    const subscription = getCapturedSubscription("perpsTrades");

    act(() => {
      subscription.listener({
        perpsTrades: {
          fillPrice: "99",
          isMaker: true,
          tradeId: "maker",
        },
      });
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toMatchObject({
      currentPrice: null,
      previousPrice: null,
      trades: [],
    });

    act(() => {
      subscription.listener({
        perpsTrades: {
          fillPrice: "100",
          isMaker: false,
          tradeId: "taker-1",
        },
      });
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toMatchObject({
      currentPrice: "100",
      previousPrice: null,
      status: "ready",
      trades: [
        {
          tradeId: "taker-1",
        },
      ],
    });

    act(() => {
      subscription.listener({
        perpsTrades: {
          fillPrice: "101",
          isMaker: false,
          tradeId: "taker-2",
        },
      });
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toMatchObject({
      currentPrice: "101",
      previousPrice: "100",
      trades: [
        {
          tradeId: "taker-2",
        },
        {
          tradeId: "taker-1",
        },
      ],
    });
  });

  it("preserves backend zero-valued live trade fields after buffering", async () => {
    const { result } = renderHook(() =>
      useLivePerpsTrades((snapshot) => snapshot, {
        perpsPairId: "BTC-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    vi.useFakeTimers();
    const subscription = getCapturedSubscription("perpsTrades");

    act(() => {
      subscription.listener({
        perpsTrades: {
          blockHeight: 0,
          fillPrice: "100",
          isMaker: false,
          tradeId: "genesis-fill",
          tradeIdx: 0,
        },
      });
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toMatchObject({
      currentPrice: "100",
      previousPrice: null,
      status: "ready",
      trades: [
        {
          blockHeight: 0,
          tradeId: "genesis-fill",
          tradeIdx: 0,
        },
      ],
    });
  });

  it("surfaces live perps trade subscription failures without dropping latest trade state", async () => {
    const { result } = renderHook(() =>
      useLivePerpsTrades((snapshot) => snapshot, {
        perpsPairId: "BTC-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    vi.useFakeTimers();
    const subscription = getCapturedSubscription("perpsTrades");

    act(() => {
      subscription.listener({
        perpsTrades: {
          fillPrice: "100",
          isMaker: false,
          tradeId: "taker-1",
        },
      });
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toMatchObject({
      currentPrice: "100",
      previousPrice: null,
      status: "ready",
      trades: [
        {
          tradeId: "taker-1",
        },
      ],
    });

    const streamError = new Error("perps trades stream failed");

    act(() => {
      subscription.onError(streamError);
    });

    expect(result.current).toMatchObject({
      currentPrice: "100",
      error: streamError,
      previousPrice: null,
      status: "error",
      trades: [
        {
          tradeId: "taker-1",
        },
      ],
    });
  });

  it("coalesces live perps trade bursts and keeps the newest fifty trades", async () => {
    const { result } = renderHook(() =>
      useLivePerpsTrades((snapshot) => snapshot, {
        perpsPairId: "ETH-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    vi.useFakeTimers();
    const subscription = getCapturedSubscription("perpsTrades");

    act(() => {
      subscription.listener({
        perpsTrades: {
          fillPrice: "2500",
          isMaker: false,
          tradeId: "seed",
        },
      });
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toMatchObject({
      currentPrice: "2500",
      previousPrice: null,
      trades: [
        {
          tradeId: "seed",
        },
      ],
    });

    act(() => {
      for (let index = 0; index < 55; index += 1) {
        subscription.listener({
          perpsTrades: {
            fillPrice: `${2600 + index}`,
            isMaker: false,
            tradeId: `burst-${index}`,
          },
        });
      }
      vi.advanceTimersByTime(500);
    });

    expect(result.current.currentPrice).toBe("2654");
    expect(result.current.previousPrice).toBe("2500");
    expect(result.current.trades).toHaveLength(50);
    expect(result.current.trades[0]).toMatchObject({ tradeId: "burst-54" });
    expect(result.current.trades.at(-1)).toMatchObject({ tradeId: "burst-5" });
    expect(result.current.trades).not.toEqual(
      expect.arrayContaining([expect.objectContaining({ tradeId: "seed" })]),
    );
  });

  it("cancels pending live perps trade batches when the stream is released", async () => {
    const { result, unmount } = renderHook(() =>
      useLivePerpsTrades((snapshot) => snapshot, {
        perpsPairId: "SOL-USD",
      }),
    );

    await waitFor(() => expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledOnce());

    vi.useFakeTimers();
    const subscription = getCapturedSubscription("perpsTrades");
    const unsubscribe = hookMocks.subscriptionsSubscribe.mock.results[0].value;
    const clearTimeoutSpy = vi.spyOn(globalThis, "clearTimeout");

    act(() => {
      subscription.listener({
        perpsTrades: {
          fillPrice: "142",
          isMaker: false,
          tradeId: "pending-batch",
        },
      });
    });

    expect(result.current.trades).toEqual([]);

    const callsBeforeUnmount = clearTimeoutSpy.mock.calls.length;

    unmount();

    expect(unsubscribe).toHaveBeenCalledOnce();
    expect(clearTimeoutSpy.mock.calls.length).toBeGreaterThan(callsBeforeUnmount);

    act(() => {
      vi.advanceTimersByTime(500);
    });

    expect(result.current.trades).toEqual([]);
  });

  it("does not start extended user-state resources without an account", () => {
    const { result } = renderHook(() =>
      usePerpsUserStateExtended((snapshot) => snapshot, {
        accountAddress: undefined,
      }),
    );

    expect(result.current).toMatchObject({
      availableMargin: null,
      equity: null,
      maintenanceMargin: null,
      status: "idle",
    });
    expect(hookMocks.publicClientQueryApp).not.toHaveBeenCalled();
    expect(hookMocks.subscriptionsSubscribe).not.toHaveBeenCalled();
  });
});
