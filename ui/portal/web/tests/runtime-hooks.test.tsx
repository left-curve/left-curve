import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useActivities } from "../../../store/src/hooks/useActivities";
import { useServiceStatus } from "../../../store/src/hooks/useServiceStatus";
import { createQueryClientWrapper, createTestQueryClient } from "./utils/query-client";

type SubscriptionOptions = {
  params: unknown;
  listener: (event: any) => void;
};

const hookMocks = vi.hoisted(() => ({
  balanceRefetch: vi.fn(),
  refreshUserStatus: vi.fn(),
  subscriptionsSubscribe: vi.fn(),
  uid: vi.fn(),
  useAccount: vi.fn(),
  useBalances: vi.fn(),
  useConfig: vi.fn(),
  usePublicClient: vi.fn(),
}));

vi.mock("@left-curve/utils", async (importOriginal) => {
  const actual = await importOriginal<object>();
  return {
    ...actual,
    uid: hookMocks.uid,
  };
});

vi.mock("../../../store/src/hooks/useAccount.js", () => ({
  useAccount: hookMocks.useAccount,
}));

vi.mock("../../../store/src/hooks/useBalances.js", () => ({
  useBalances: hookMocks.useBalances,
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: hookMocks.useConfig,
}));

vi.mock("../../../store/src/hooks/usePublicClient.js", () => ({
  usePublicClient: hookMocks.usePublicClient,
}));

vi.mock("../../../store/src/hooks/useStorage.js", async () => {
  const React = await import("react");

  return {
    useStorage: <T,>(_key: string, options: { initialValue?: T } = {}) =>
      React.useState<T>((options.initialValue ?? {}) as T),
  };
});

class MockEmitter {
  on = vi.fn();
  off = vi.fn();
}

function mockHealthyUpEndpoint() {
  return vi.spyOn(globalThis, "fetch").mockResolvedValue({
    ok: true,
    json: vi.fn().mockResolvedValue({ is_running: true }),
  } as unknown as Response);
}

function mockPausedUpEndpoint() {
  return vi.spyOn(globalThis, "fetch").mockResolvedValue({
    ok: true,
    json: vi.fn().mockResolvedValue({ is_running: false }),
  } as unknown as Response);
}

function mockFailingUpEndpoint() {
  return vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("status unavailable"));
}

function mockHttpErrorUpEndpoint() {
  return vi.spyOn(globalThis, "fetch").mockResolvedValue({
    ok: false,
    json: vi.fn(),
  } as unknown as Response);
}

function mockUpEndpointResponses(...responses: boolean[]) {
  let responseIndex = 0;

  return vi.spyOn(globalThis, "fetch").mockImplementation(async () => {
    const isRunning = responses[Math.min(responseIndex, responses.length - 1)];
    responseIndex += 1;

    return {
      ok: true,
      json: vi.fn().mockResolvedValue({ is_running: isRunning }),
    } as unknown as Response;
  });
}

function getSubscription(topic: string) {
  const entry = hookMocks.subscriptionsSubscribe.mock.calls.find(
    ([candidate]) => candidate === topic,
  );
  if (!entry) throw new Error(`No ${topic} subscription`);
  return entry[1] as SubscriptionOptions;
}

function getLatestSubscription(topic: string) {
  const entry = [...hookMocks.subscriptionsSubscribe.mock.calls]
    .reverse()
    .find(([candidate]) => candidate === topic);
  if (!entry) throw new Error(`No ${topic} subscription`);
  return entry[1] as SubscriptionOptions;
}

describe("runtime hooks", () => {
  let uidIndex = 0;
  let publicClient: {
    subscribe: {
      emitter: MockEmitter;
      getClientStatus: () => { isConnected: boolean };
    };
  };
  let isConnected: boolean;

  beforeEach(() => {
    uidIndex = 0;
    isConnected = true;
    publicClient = {
      subscribe: {
        emitter: new MockEmitter(),
        getClientStatus: () => ({ isConnected }),
      },
    };

    hookMocks.uid.mockImplementation(() => {
      uidIndex += 1;
      return `activity-${uidIndex}`;
    });
    hookMocks.usePublicClient.mockReturnValue(publicClient);
    hookMocks.useBalances.mockReturnValue({
      refetch: hookMocks.balanceRefetch,
    });
    hookMocks.useAccount.mockReturnValue({
      account: {
        address: "0x6d61696e00000000000000000000000000000000",
        owner: 7,
      },
      accounts: [
        { address: "0x6d61696e00000000000000000000000000000000" },
        { address: "0x6261636b75700000000000000000000000000000" },
      ],
      refreshUserStatus: hookMocks.refreshUserStatus,
      userIndex: 7,
      userStatus: "inactive",
    });
    hookMocks.useConfig.mockReturnValue({
      subscriptions: {
        subscribe: hookMocks.subscriptionsSubscribe,
      },
    });
    hookMocks.subscriptionsSubscribe.mockImplementation(() => vi.fn());
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.clearAllMocks();
  });

  it("reports healthy service status after the delayed websocket check and chain check", async () => {
    mockHealthyUpEndpoint();

    const { result } = renderHook(
      () =>
        useServiceStatus({
          upUrl: "https://status.test/up",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(
      () =>
        expect(result.current).toMatchObject({
          chainStatus: "success",
          globalStatus: "success",
          isChainPaused: false,
          isReady: true,
          transportMode: "ws",
          wsStatus: "success",
        }),
      { timeout: 2500 },
    );

    expect(globalThis.fetch).toHaveBeenCalledWith("https://status.test/up");
    expect(publicClient.subscribe.emitter.on).toHaveBeenCalledWith(
      "transport-mode",
      expect.any(Function),
    );
  });

  it("surfaces websocket outages as a global warning while the chain check is healthy", async () => {
    isConnected = false;
    mockHealthyUpEndpoint();

    const { result } = renderHook(
      () =>
        useServiceStatus({
          upUrl: "https://status.test/up",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(
      () =>
        expect(result.current).toMatchObject({
          chainStatus: "success",
          globalStatus: "warning",
          isChainPaused: false,
          isReady: true,
          transportMode: "reconnecting",
          wsStatus: "error",
        }),
      { timeout: 2500 },
    );
    expect(globalThis.fetch).toHaveBeenCalledWith("https://status.test/up");
  });

  it("keeps service status usable when websocket helpers are unavailable", async () => {
    hookMocks.usePublicClient.mockReturnValue({
      subscribe: {
        getClientStatus: () => ({ isConnected: false }),
      },
    });
    mockHealthyUpEndpoint();

    const { result } = renderHook(
      () =>
        useServiceStatus({
          upUrl: "https://status.test/up",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(
      () =>
        expect(result.current).toMatchObject({
          chainStatus: "success",
          globalStatus: "warning",
          isChainPaused: false,
          isReady: true,
          transportMode: "reconnecting",
          wsStatus: "error",
        }),
      { timeout: 2500 },
    );

    expect(globalThis.fetch).toHaveBeenCalledWith("https://status.test/up");
    expect(publicClient.subscribe.emitter.on).not.toHaveBeenCalled();
  });

  it("cleans up service status transport listeners and skips chain checks without an up URL", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    const { unmount } = renderHook(() => useServiceStatus(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() =>
      expect(publicClient.subscribe.emitter.on).toHaveBeenCalledWith(
        "transport-mode",
        expect.any(Function),
      ),
    );

    const [, listener] = publicClient.subscribe.emitter.on.mock.calls.find(
      ([event]) => event === "transport-mode",
    )!;

    unmount();

    expect(publicClient.subscribe.emitter.off).toHaveBeenCalledWith("transport-mode", listener);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("requires three failed chain checks before surfacing paused chain status", async () => {
    mockPausedUpEndpoint();
    const queryClient = createTestQueryClient();

    const { result } = renderHook(
      () =>
        useServiceStatus({
          upUrl: "https://status.test/up",
        }),
      {
        wrapper: createQueryClientWrapper(queryClient),
      },
    );

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });
    expect(globalThis.fetch).toHaveBeenCalledTimes(2);
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        chainStatus: "error",
        globalStatus: "error",
        isChainPaused: true,
      }),
    );
    expect(globalThis.fetch).toHaveBeenCalledTimes(3);
  });

  it("requires three failed health requests before treating the chain as paused", async () => {
    mockFailingUpEndpoint();
    const queryClient = createTestQueryClient();

    const { result } = renderHook(
      () =>
        useServiceStatus({
          upUrl: "https://status.test/up",
        }),
      {
        wrapper: createQueryClientWrapper(queryClient),
      },
    );

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });
    expect(globalThis.fetch).toHaveBeenCalledTimes(2);
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        chainStatus: "error",
        globalStatus: "error",
        isChainPaused: true,
      }),
    );
    expect(globalThis.fetch).toHaveBeenCalledTimes(3);
  });

  it("requires three non-OK health responses before treating the chain as paused", async () => {
    mockHttpErrorUpEndpoint();
    const queryClient = createTestQueryClient();

    const { result } = renderHook(
      () =>
        useServiceStatus({
          upUrl: "https://status.test/up",
        }),
      {
        wrapper: createQueryClientWrapper(queryClient),
      },
    );

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });
    expect(globalThis.fetch).toHaveBeenCalledTimes(2);
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        chainStatus: "error",
        globalStatus: "error",
        isChainPaused: true,
      }),
    );
    expect(globalThis.fetch).toHaveBeenCalledTimes(3);
  });

  it("resets paused-chain detection after an intermittent healthy backend check", async () => {
    mockUpEndpointResponses(false, false, true, false, false, false);
    const queryClient = createTestQueryClient();

    const { result } = renderHook(
      () =>
        useServiceStatus({
          upUrl: "https://status.test/up",
        }),
      {
        wrapper: createQueryClientWrapper(queryClient),
      },
    );

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });
    expect(globalThis.fetch).toHaveBeenCalledTimes(2);
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });
    expect(globalThis.fetch).toHaveBeenCalledTimes(3);
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });
    expect(globalThis.fetch).toHaveBeenCalledTimes(4);
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });
    expect(globalThis.fetch).toHaveBeenCalledTimes(5);
    expect(result.current.chainStatus).toBe("success");

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["chain_status"] });
    });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        chainStatus: "error",
        globalStatus: "error",
        isChainPaused: true,
      }),
    );
    expect(globalThis.fetch).toHaveBeenCalledTimes(6);
  });

  it("subscribes to account and event activity streams and records visible activity state", async () => {
    const queryClient = createTestQueryClient();
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

    const { result } = renderHook(() => useActivities(), {
      wrapper: createQueryClientWrapper(queryClient),
    });

    let stopActivities: (() => void) | undefined;
    act(() => {
      stopActivities = result.current.startActivities();
    });

    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "account",
      expect.objectContaining({
        params: { userIndex: 7 },
      }),
    );
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "eventsByAddresses",
      expect.objectContaining({
        params: {
          addresses: [
            "0x6d61696e00000000000000000000000000000000",
            "0x6261636b75700000000000000000000000000000",
          ],
        },
      }),
    );

    act(() => {
      getSubscription("account").listener({
        accounts: [
          {
            accountIndex: 2,
            address: "0x6e65770000000000000000000000000000000000",
            createdAt: "2026-06-08T10:00:00Z",
            createdBlockHeight: 12,
          },
        ],
      });
    });

    await waitFor(() => expect(result.current.userActivities).toHaveLength(1));
    expect(result.current.userActivities[0]).toMatchObject({
      blockHeight: 12,
      data: {
        accountIndex: 2,
        address: "0x6e65770000000000000000000000000000000000",
      },
      id: "activity-1",
      seen: false,
      type: "account",
    });

    act(() => {
      getSubscription("eventsByAddresses").listener([
        {
          blockHeight: 13,
          createdAt: "2026-06-08T10:01:00Z",
          data: {
            contract_event: {
              data: {
                coins: { "bridge/usdc": "100" },
                from: "0x6d61696e00000000000000000000000000000000",
                to: "0x667269656e640000000000000000000000000000",
                user: "0x6d61696e00000000000000000000000000000000",
              },
              type: "sent",
            },
          },
          transaction: {
            hash: "0xtransfer",
          },
        },
        {
          blockHeight: 14,
          createdAt: "2026-06-08T10:02:00Z",
          data: {
            contract_event: {
              data: {
                orderId: "order-1",
                pairId: "perp/btcusd",
              },
              type: "order_filled",
            },
          },
          transaction: {
            hash: "0xorder",
          },
        },
      ]);
    });

    await waitFor(() => expect(result.current.userActivities).toHaveLength(3));

    expect(hookMocks.balanceRefetch).toHaveBeenCalledTimes(2);
    expect(hookMocks.refreshUserStatus).toHaveBeenCalledOnce();
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x6d61696e00000000000000000000000000000000"],
    });
    expect(result.current.unseenCount).toBe(3);
    expect(result.current.hasActivities).toBe(true);
    expect(result.current.userActivities[1]).toMatchObject({
      blockHeight: 13,
      data: {
        coins: { "bridge/usdc": "100" },
        fromAddress: "0x6d61696e00000000000000000000000000000000",
        toAddress: "0x667269656e640000000000000000000000000000",
        type: "sent",
      },
      id: "activity-2",
      txHash: "0xtransfer",
      type: "transfer",
    });
    expect(result.current.userActivities[2]).toMatchObject({
      blockHeight: 14,
      data: {
        orderId: "order-1",
        pairId: "perp/btcusd",
      },
      id: "activity-3",
      txHash: "0xorder",
      type: "perpOrderFilled",
    });

    act(() => {
      result.current.markAllSeen();
    });

    await waitFor(() => expect(result.current.unseenCount).toBe(0));
    expect(result.current.userActivities.every((activity) => activity.seen)).toBe(true);

    act(() => {
      result.current.deleteActivityRecord("activity-2");
    });

    await waitFor(() => expect(result.current.totalActivities).toBe(2));
    expect(result.current.userActivities.map((activity) => activity.id)).toEqual([
      "activity-1",
      "activity-3",
    ]);

    act(() => {
      stopActivities?.();
    });

    const accountUnsubscribe = hookMocks.subscriptionsSubscribe.mock.results[0].value;
    const eventsUnsubscribe = hookMocks.subscriptionsSubscribe.mock.results[1].value;
    expect(eventsUnsubscribe).toHaveBeenCalledOnce();
    expect(accountUnsubscribe).toHaveBeenCalledOnce();
  });

  it("records only in-account transfer activity with non-empty coins", async () => {
    const { result } = renderHook(() => useActivities(), {
      wrapper: createQueryClientWrapper(),
    });

    act(() => {
      result.current.startActivities();
    });

    act(() => {
      getSubscription("eventsByAddresses").listener([
        {
          blockHeight: 30,
          createdAt: "2026-06-08T12:00:00Z",
          data: {
            contract_event: {
              data: {
                coins: { "bridge/usdc": "42" },
                user: "0x6261636b75700000000000000000000000000000",
              },
              type: "received",
            },
          },
          transaction: {
            hash: "0x7265636569766564000000000000000000000000000000000000000000000000",
          },
        },
        {
          blockHeight: 31,
          createdAt: "2026-06-08T12:01:00Z",
          data: {
            contract_event: {
              data: {
                coins: { "bridge/usdc": "10" },
                from: "0x65787465726e616c000000000000000000000000",
                to: "0x667269656e640000000000000000000000000000",
                user: "0x65787465726e616c000000000000000000000000",
              },
              type: "sent",
            },
          },
          transaction: {
            hash: "0x65787465726e616c73656e7400000000000000000000000000000000000000",
          },
        },
        {
          blockHeight: 32,
          createdAt: "2026-06-08T12:02:00Z",
          data: {
            contract_event: {
              data: {
                coins: {},
                from: "0x6d61696e00000000000000000000000000000000",
                to: "0x667269656e640000000000000000000000000000",
                user: "0x6d61696e00000000000000000000000000000000",
              },
              type: "sent",
            },
          },
          transaction: {
            hash: "0x656d707479636f696e73000000000000000000000000000000000000000000",
          },
        },
      ]);
    });

    await waitFor(() => expect(result.current.userActivities).toHaveLength(1));
    expect(result.current.userActivities[0]).toMatchObject({
      blockHeight: 30,
      data: {
        coins: { "bridge/usdc": "42" },
        fromAddress: "0x6261636b75700000000000000000000000000000",
        toAddress: "0x6261636b75700000000000000000000000000000",
        type: "received",
      },
      id: "activity-1",
      seen: false,
      txHash: "0x7265636569766564000000000000000000000000000000000000000000000000",
      type: "transfer",
    });
    expect(result.current.unseenCount).toBe(1);
  });

  it("keeps persisted activity state isolated by backend user index", async () => {
    const { rerender, result } = renderHook(() => useActivities(), {
      wrapper: createQueryClientWrapper(),
    });

    act(() => {
      result.current.startActivities();
    });

    act(() => {
      getSubscription("eventsByAddresses").listener([
        {
          blockHeight: 40,
          createdAt: "2026-06-08T13:00:00Z",
          data: {
            contract_event: {
              data: {
                coins: { "bridge/usdc": "15" },
                from: "0x667269656e640000000000000000000000000000",
                to: "0x6d61696e00000000000000000000000000000000",
                user: "0x6d61696e00000000000000000000000000000000",
              },
              type: "received",
            },
          },
          transaction: {
            hash: "0x7573657237616374697669747900000000000000000000000000000000000000",
          },
        },
      ]);
    });

    await waitFor(() => expect(result.current.userActivities).toHaveLength(1));
    expect(result.current.userActivities[0]).toMatchObject({
      blockHeight: 40,
      id: "activity-1",
      type: "transfer",
    });

    hookMocks.useAccount.mockReturnValue({
      account: {
        address: "0x6f74686572000000000000000000000000000000",
        owner: 8,
      },
      accounts: [{ address: "0x6f74686572000000000000000000000000000000" }],
      refreshUserStatus: hookMocks.refreshUserStatus,
      userIndex: 8,
      userStatus: "inactive",
    });

    rerender();

    expect(result.current.userActivities).toEqual([]);
    expect(result.current.totalActivities).toBe(0);
    expect(result.current.unseenCount).toBe(0);

    act(() => {
      result.current.startActivities();
    });

    expect(hookMocks.subscriptionsSubscribe).toHaveBeenLastCalledWith(
      "eventsByAddresses",
      expect.objectContaining({
        params: {
          addresses: ["0x6f74686572000000000000000000000000000000"],
        },
      }),
    );

    act(() => {
      getLatestSubscription("eventsByAddresses").listener([
        {
          blockHeight: 41,
          createdAt: "2026-06-08T13:01:00Z",
          data: {
            contract_event: {
              data: {
                coins: { "bridge/usdc": "22" },
                from: "0x6f74686572000000000000000000000000000000",
                to: "0x667269656e640000000000000000000000000000",
                user: "0x6f74686572000000000000000000000000000000",
              },
              type: "sent",
            },
          },
          transaction: {
            hash: "0x7573657238616374697669747900000000000000000000000000000000000000",
          },
        },
      ]);
    });

    await waitFor(() => expect(result.current.userActivities).toHaveLength(1));
    expect(result.current.userActivities[0]).toMatchObject({
      blockHeight: 41,
      id: "activity-2",
      type: "transfer",
    });

    hookMocks.useAccount.mockReturnValue({
      account: {
        address: "0x6d61696e00000000000000000000000000000000",
        owner: 7,
      },
      accounts: [
        { address: "0x6d61696e00000000000000000000000000000000" },
        { address: "0x6261636b75700000000000000000000000000000" },
      ],
      refreshUserStatus: hookMocks.refreshUserStatus,
      userIndex: 7,
      userStatus: "inactive",
    });

    rerender();

    expect(result.current.userActivities).toMatchObject([
      {
        blockHeight: 40,
        id: "activity-1",
        type: "transfer",
      },
    ]);
    expect(result.current.totalActivities).toBe(1);
    expect(result.current.unseenCount).toBe(1);
  });

  it("refreshes balances without refreshing active user status for transfer activity", async () => {
    hookMocks.useAccount.mockReturnValue({
      account: {
        address: "0x6d61696e00000000000000000000000000000000",
        owner: 7,
      },
      accounts: [
        { address: "0x6d61696e00000000000000000000000000000000" },
        { address: "0x6261636b75700000000000000000000000000000" },
      ],
      refreshUserStatus: hookMocks.refreshUserStatus,
      userIndex: 7,
      userStatus: "active",
    });

    const { result } = renderHook(() => useActivities(), {
      wrapper: createQueryClientWrapper(),
    });

    act(() => {
      result.current.startActivities();
    });

    act(() => {
      getSubscription("eventsByAddresses").listener([
        {
          blockHeight: 33,
          createdAt: "2026-06-08T12:03:00Z",
          data: {
            contract_event: {
              data: {
                coins: { "bridge/usdc": "75" },
                from: "0x667269656e640000000000000000000000000000",
                to: "0x6d61696e00000000000000000000000000000000",
                user: "0x6d61696e00000000000000000000000000000000",
              },
              type: "received",
            },
          },
          transaction: {
            hash: "0x6163746976657265636569766564000000000000000000000000000000000000",
          },
        },
      ]);
    });

    await waitFor(() => expect(result.current.userActivities).toHaveLength(1));

    expect(hookMocks.balanceRefetch).toHaveBeenCalledOnce();
    expect(hookMocks.refreshUserStatus).not.toHaveBeenCalled();
    expect(result.current.userActivities[0]).toMatchObject({
      blockHeight: 33,
      data: {
        coins: { "bridge/usdc": "75" },
        fromAddress: "0x667269656e640000000000000000000000000000",
        toAddress: "0x6d61696e00000000000000000000000000000000",
        type: "received",
      },
      id: "activity-1",
      txHash: "0x6163746976657265636569766564000000000000000000000000000000000000",
      type: "transfer",
    });
  });

  it("records backend perps activity variants and ignores unsupported activity events", async () => {
    const queryClient = createTestQueryClient();
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

    const { result } = renderHook(() => useActivities(), {
      wrapper: createQueryClientWrapper(queryClient),
    });

    act(() => {
      result.current.startActivities();
    });

    act(() => {
      getSubscription("eventsByAddresses").listener([
        {
          blockHeight: 20,
          createdAt: "2026-06-08T11:00:00Z",
          data: {
            message: "not a contract event",
          },
          transaction: {
            hash: "0x6e6f6e636f6e7472616374000000000000000000000000000000000000000000",
          },
        },
        {
          blockHeight: 21,
          createdAt: "2026-06-08T11:01:00Z",
          data: {
            contract_event: {
              data: {
                ignored: true,
              },
              type: "unknown_event",
            },
          },
          transaction: {
            hash: "0x756e6b6e6f776e00000000000000000000000000000000000000000000000000",
          },
        },
        {
          blockHeight: 22,
          createdAt: "2026-06-08T11:02:00Z",
          data: {
            contract_event: {
              data: {
                closing_size: "0",
                fee: "0.2",
                fill_price: "60000",
                fill_size: "0.5",
                is_maker: false,
                opening_size: "0.5",
                order_id: "order-fill-9",
                pair_id: "perp/btcusd",
                realized_funding: "0.01",
                realized_pnl: "18.5",
                user: "0x6d61696e00000000000000000000000000000000",
              },
              type: "order_filled",
            },
          },
          transaction: {
            hash: "0x6f7264657266696c6c6564000000000000000000000000000000000000000000",
          },
        },
        {
          blockHeight: 23,
          createdAt: "2026-06-08T11:03:00Z",
          data: {
            contract_event: {
              data: {
                adl_price: "3000",
                adl_realized_funding: "-0.02",
                adl_realized_pnl: "-42",
                adl_size: "-1.2",
                pair_id: "perp/ethusd",
                user: "0x6261636b75700000000000000000000000000000",
              },
              type: "liquidated",
            },
          },
          transaction: {
            hash: "0x6c69717569646174656400000000000000000000000000000000000000000000",
          },
        },
        {
          blockHeight: 24,
          createdAt: "2026-06-08T11:04:00Z",
          data: {
            contract_event: {
              data: {
                closing_size: "0.75",
                fill_price: "2.25",
                pair_id: "perp/atomusd",
                realized_funding: null,
                realized_pnl: "0",
                user: "0x6d61696e00000000000000000000000000000000",
              },
              type: "deleveraged",
            },
          },
          transaction: {
            hash: "0x64656c6576657261676564000000000000000000000000000000000000000000",
          },
        },
      ]);
    });

    await waitFor(() => expect(result.current.userActivities).toHaveLength(3));

    expect(hookMocks.balanceRefetch).toHaveBeenCalledTimes(3);
    expect(hookMocks.refreshUserStatus).not.toHaveBeenCalled();
    expect(invalidateQueries).toHaveBeenCalledOnce();
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x6d61696e00000000000000000000000000000000"],
    });
    expect(result.current.unseenCount).toBe(3);
    expect(result.current.userActivities).toMatchObject([
      {
        blockHeight: 22,
        data: {
          fill_price: "60000",
          fill_size: "0.5",
          is_maker: false,
          order_id: "order-fill-9",
          pair_id: "perp/btcusd",
          realized_funding: "0.01",
          realized_pnl: "18.5",
        },
        id: "activity-1",
        seen: false,
        txHash: "0x6f7264657266696c6c6564000000000000000000000000000000000000000000",
        type: "perpOrderFilled",
      },
      {
        blockHeight: 23,
        data: {
          adl_price: "3000",
          adl_realized_funding: "-0.02",
          adl_realized_pnl: "-42",
          adl_size: "-1.2",
          pair_id: "perp/ethusd",
        },
        id: "activity-2",
        seen: false,
        txHash: "0x6c69717569646174656400000000000000000000000000000000000000000000",
        type: "perpLiquidated",
      },
      {
        blockHeight: 24,
        data: {
          closing_size: "0.75",
          fill_price: "2.25",
          pair_id: "perp/atomusd",
          realized_funding: null,
          realized_pnl: "0",
        },
        id: "activity-3",
        seen: false,
        txHash: "0x64656c6576657261676564000000000000000000000000000000000000000000",
        type: "perpDeleveraged",
      },
    ]);
  });

  it("does not start activity subscriptions without an active account context", () => {
    hookMocks.useAccount.mockReturnValue({
      account: null,
      accounts: [],
      userIndex: undefined,
      userStatus: "inactive",
    });

    const { result } = renderHook(() => useActivities(), {
      wrapper: createQueryClientWrapper(),
    });

    act(() => {
      result.current.startActivities();
    });

    expect(result.current.userActivities).toEqual([]);
    expect(hookMocks.subscriptionsSubscribe).not.toHaveBeenCalled();
  });

  it("starts activity subscriptions for backend user index zero", async () => {
    hookMocks.useAccount.mockReturnValue({
      account: {
        address: "0x7a65726f00000000000000000000000000000000",
        owner: 0,
      },
      accounts: [{ address: "0x7a65726f00000000000000000000000000000000" }],
      refreshUserStatus: hookMocks.refreshUserStatus,
      userIndex: 0,
      userStatus: "inactive",
    });

    const { result } = renderHook(() => useActivities(), {
      wrapper: createQueryClientWrapper(),
    });

    act(() => {
      result.current.startActivities();
    });

    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "account",
      expect.objectContaining({
        params: { userIndex: 0 },
      }),
    );
    expect(hookMocks.subscriptionsSubscribe).toHaveBeenCalledWith(
      "eventsByAddresses",
      expect.objectContaining({
        params: {
          addresses: ["0x7a65726f00000000000000000000000000000000"],
        },
      }),
    );

    act(() => {
      getSubscription("eventsByAddresses").listener([
        {
          blockHeight: 50,
          createdAt: "2026-06-08T14:00:00Z",
          data: {
            contract_event: {
              data: {
                coins: { "bridge/usdc": "11" },
                from: "0x667269656e640000000000000000000000000000",
                to: "0x7a65726f00000000000000000000000000000000",
                user: "0x7a65726f00000000000000000000000000000000",
              },
              type: "received",
            },
          },
          transaction: {
            hash: "0x7573657230726563656976656400000000000000000000000000000000000000",
          },
        },
      ]);
    });

    await waitFor(() => expect(result.current.userActivities).toHaveLength(1));
    expect(result.current.userActivities[0]).toMatchObject({
      blockHeight: 50,
      data: {
        coins: { "bridge/usdc": "11" },
        fromAddress: "0x667269656e640000000000000000000000000000",
        toAddress: "0x7a65726f00000000000000000000000000000000",
        type: "received",
      },
      id: "activity-1",
      seen: false,
      txHash: "0x7573657230726563656976656400000000000000000000000000000000000000",
      type: "transfer",
    });
  });
});
