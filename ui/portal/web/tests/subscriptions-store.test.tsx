import { afterEach, describe, expect, it, vi } from "vitest";

import { subscriptionsStore } from "../../../store/src/subscriptions";

function createSubscriptionClient() {
  const client = {
    accountSubscription: vi.fn(() => vi.fn()),
    allPerpsPairStatsSubscription: vi.fn(() => vi.fn()),
    blockSubscription: vi.fn(() => vi.fn()),
    eventsByAddressesSubscription: vi.fn(() => vi.fn()),
    eventsSubscription: vi.fn(() => vi.fn()),
    perpsCandlesSubscription: vi.fn(() => vi.fn()),
    perpsTradesSubscription: vi.fn(() => vi.fn()),
    queryAppSubscription: vi.fn(() => vi.fn()),
    transferSubscription: vi.fn(() => vi.fn()),
  };

  return client;
}

describe("subscriptions store", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("shares one backend executor for matching subscription keys and params", () => {
    const executorUnsubscribe = vi.fn();
    let emitEvents: ((event: { events: unknown[] }) => void) | undefined;
    const client = createSubscriptionClient();
    client.eventsSubscription.mockImplementation(({ next }) => {
      emitEvents = next;
      return executorUnsubscribe;
    });
    const store = subscriptionsStore(client as never);
    const firstListener = vi.fn();
    const secondListener = vi.fn();
    const params = {
      filter: [{ type: "transfer" }],
      sinceBlockHeight: 12,
    };

    const unsubscribeFirst = store.subscribe("events", {
      listener: firstListener,
      params,
    });
    const unsubscribeSecond = store.subscribe("events", {
      listener: secondListener,
      params,
    });

    expect(client.eventsSubscription).toHaveBeenCalledOnce();
    expect(client.eventsSubscription).toHaveBeenCalledWith({
      ...params,
      error: expect.any(Function),
      next: expect.any(Function),
    });

    emitEvents?.({
      events: [{ id: "event-1" }],
    });

    expect(firstListener).toHaveBeenCalledWith([{ id: "event-1" }]);
    expect(secondListener).toHaveBeenCalledWith([{ id: "event-1" }]);

    unsubscribeFirst();
    expect(executorUnsubscribe).not.toHaveBeenCalled();

    emitEvents?.({
      events: [{ id: "event-2" }],
    });

    expect(firstListener).toHaveBeenCalledTimes(1);
    expect(secondListener).toHaveBeenLastCalledWith([{ id: "event-2" }]);

    unsubscribeSecond();
    expect(executorUnsubscribe).toHaveBeenCalledOnce();
  });

  it("keeps separate backend executors and emit channels for different params", () => {
    const firstUnsubscribe = vi.fn();
    const secondUnsubscribe = vi.fn();
    const client = createSubscriptionClient();
    client.transferSubscription
      .mockReturnValueOnce(firstUnsubscribe)
      .mockReturnValueOnce(secondUnsubscribe);
    const store = subscriptionsStore(client as never);
    const aliceListener = vi.fn();
    const bobListener = vi.fn();
    const aliceParams = {
      sinceBlockHeight: 1,
      username: "alice",
    };
    const bobParams = {
      sinceBlockHeight: 1,
      username: "bob",
    };

    const unsubscribeAlice = store.subscribe("transfer", {
      listener: aliceListener,
      params: aliceParams,
    });
    const unsubscribeBob = store.subscribe("transfer", {
      listener: bobListener,
      params: bobParams,
    });

    expect(client.transferSubscription).toHaveBeenCalledTimes(2);

    store.emit(
      {
        key: "transfer",
        params: aliceParams,
      },
      {
        transfers: [{ id: "alice-transfer" }],
      } as never,
    );

    expect(aliceListener).toHaveBeenCalledWith({
      transfers: [{ id: "alice-transfer" }],
    });
    expect(bobListener).not.toHaveBeenCalled();

    unsubscribeAlice();
    expect(firstUnsubscribe).toHaveBeenCalledOnce();
    expect(secondUnsubscribe).not.toHaveBeenCalled();

    unsubscribeBob();
    expect(secondUnsubscribe).toHaveBeenCalledOnce();
  });

  it("maps backend subscription envelopes to frontend listener payloads", () => {
    let emitBlock: ((event: { block: unknown }) => void) | undefined;
    let emitEventsByAddress: ((event: { eventByAddresses: unknown[] }) => void) | undefined;
    let emitQueryApp: ((event: { queryApp: unknown }) => void) | undefined;
    const client = createSubscriptionClient();
    client.blockSubscription.mockImplementation(({ next }) => {
      emitBlock = next;
      return vi.fn();
    });
    client.eventsByAddressesSubscription.mockImplementation(({ next }) => {
      emitEventsByAddress = next;
      return vi.fn();
    });
    client.queryAppSubscription.mockImplementation(({ next }) => {
      emitQueryApp = next;
      return vi.fn();
    });
    const store = subscriptionsStore(client as never);
    const blockListener = vi.fn();
    const eventsByAddressListener = vi.fn();
    const queryAppListener = vi.fn();

    store.subscribe("block", {
      listener: blockListener,
    });
    store.subscribe("eventsByAddresses", {
      listener: eventsByAddressListener,
      params: {
        addresses: ["0x737562736372697074696f6e2d61646472"],
        sinceBlockHeight: 8,
      },
    });
    store.subscribe("queryApp", {
      listener: queryAppListener,
      params: {
        request: {
          key: "perps_state",
        },
      } as never,
    });

    emitBlock?.({
      block: {
        height: 9,
      },
    });
    emitEventsByAddress?.({
      eventByAddresses: [{ id: "address-event" }],
    });
    emitQueryApp?.({
      queryApp: {
        blockHeight: 9,
        response: {
          state: "ok",
        },
      },
    });

    expect(blockListener).toHaveBeenCalledWith({
      height: 9,
    });
    expect(eventsByAddressListener).toHaveBeenCalledWith([{ id: "address-event" }]);
    expect(queryAppListener).toHaveBeenCalledWith({
      blockHeight: 9,
      response: {
        state: "ok",
      },
    });
  });

  it("delivers backend callbacks fired during subscription startup", () => {
    const startupError = new Error("startup subscription failed");
    const executorUnsubscribe = vi.fn();
    const globalErrorHandler = vi.fn();
    const listenerErrorHandler = vi.fn();
    const client = createSubscriptionClient();
    client.blockSubscription.mockImplementation(({ error, next }) => {
      next({
        block: {
          height: 42,
        },
      });
      error(startupError);
      return executorUnsubscribe;
    });
    const store = subscriptionsStore(client as never, {
      onError: globalErrorHandler,
    });
    const listener = vi.fn();

    const unsubscribe = store.subscribe("block", {
      listener,
      onError: listenerErrorHandler,
    });

    expect(listener).toHaveBeenCalledWith({
      height: 42,
    });
    expect(globalErrorHandler).toHaveBeenCalledWith(startupError);
    expect(listenerErrorHandler).toHaveBeenCalledWith(startupError);

    unsubscribe();
    expect(executorUnsubscribe).toHaveBeenCalledOnce();
  });

  it("passes account, transfer, perps, and stats subscription payloads through listeners", () => {
    let emitAccount: ((event: { accounts: unknown[] }) => void) | undefined;
    let emitTransfer: ((event: { transfers: unknown[] }) => void) | undefined;
    let emitPerpsCandles: ((event: { perpsCandles: unknown[] }) => void) | undefined;
    let emitPerpsTrades: ((event: { perpsTrades: unknown }) => void) | undefined;
    let emitAllPairStats: ((event: { allPerpsPairStats: unknown[] }) => void) | undefined;
    const client = createSubscriptionClient();
    client.accountSubscription.mockImplementation(({ next }) => {
      emitAccount = next;
      return vi.fn();
    });
    client.transferSubscription.mockImplementation(({ next }) => {
      emitTransfer = next;
      return vi.fn();
    });
    client.perpsCandlesSubscription.mockImplementation(({ next }) => {
      emitPerpsCandles = next;
      return vi.fn();
    });
    client.perpsTradesSubscription.mockImplementation(({ next }) => {
      emitPerpsTrades = next;
      return vi.fn();
    });
    client.allPerpsPairStatsSubscription.mockImplementation(({ next }) => {
      emitAllPairStats = next;
      return vi.fn();
    });
    const store = subscriptionsStore(client as never);
    const accountListener = vi.fn();
    const transferListener = vi.fn();
    const candlesListener = vi.fn();
    const tradesListener = vi.fn();
    const statsListener = vi.fn();

    store.subscribe("account", {
      listener: accountListener,
      params: {
        sinceBlockHeight: 12,
        userIndex: 7,
      },
    });
    store.subscribe("transfer", {
      listener: transferListener,
      params: {
        sinceBlockHeight: 13,
        username: "alice",
      },
    });
    store.subscribe("perpsCandles", {
      listener: candlesListener,
      params: {
        interval: "1m",
        pairId: "BTC-USD",
      },
    });
    store.subscribe("perpsTrades", {
      listener: tradesListener,
      params: {
        pairId: "BTC-USD",
      },
    });
    store.subscribe("allPerpsPairStats", {
      listener: statsListener,
      params: {
        httpInterval: 5_000,
      },
    });

    expect(client.accountSubscription).toHaveBeenCalledWith({
      error: expect.any(Function),
      next: expect.any(Function),
      sinceBlockHeight: 12,
      userIndex: 7,
    });
    expect(client.transferSubscription).toHaveBeenCalledWith({
      error: expect.any(Function),
      next: expect.any(Function),
      sinceBlockHeight: 13,
      username: "alice",
    });
    expect(client.perpsCandlesSubscription).toHaveBeenCalledWith({
      error: expect.any(Function),
      interval: "1m",
      next: expect.any(Function),
      pairId: "BTC-USD",
    });
    expect(client.perpsTradesSubscription).toHaveBeenCalledWith({
      error: expect.any(Function),
      next: expect.any(Function),
      pairId: "BTC-USD",
    });
    expect(client.allPerpsPairStatsSubscription).toHaveBeenCalledWith({
      error: expect.any(Function),
      httpInterval: 5_000,
      next: expect.any(Function),
    });

    emitAccount?.({
      accounts: [{ address: "0x6163636f756e7400000000000000000000000000" }],
    });
    emitTransfer?.({
      transfers: [{ hash: "0x7472616e73666572000000000000000000000000" }],
    });
    emitPerpsCandles?.({
      perpsCandles: [{ close: "101", open: "100" }],
    });
    emitPerpsTrades?.({
      perpsTrades: { price: "101", size: "2" },
    });
    emitAllPairStats?.({
      allPerpsPairStats: [{ pairId: "BTC-USD", volume: "1000" }],
    });

    expect(accountListener).toHaveBeenCalledWith({
      accounts: [{ address: "0x6163636f756e7400000000000000000000000000" }],
    });
    expect(transferListener).toHaveBeenCalledWith({
      transfers: [{ hash: "0x7472616e73666572000000000000000000000000" }],
    });
    expect(candlesListener).toHaveBeenCalledWith({
      perpsCandles: [{ close: "101", open: "100" }],
    });
    expect(tradesListener).toHaveBeenCalledWith({
      perpsTrades: { price: "101", size: "2" },
    });
    expect(statsListener).toHaveBeenCalledWith({
      allPerpsPairStats: [{ pairId: "BTC-USD", volume: "1000" }],
    });
  });

  it("dispatches submit transaction events as a local in-memory channel", () => {
    const client = createSubscriptionClient();
    const store = subscriptionsStore(client as never);
    const listener = vi.fn();

    const unsubscribe = store.subscribe("submitTx", {
      listener,
    });

    store.emit(
      {
        key: "submitTx",
      },
      {
        status: "pending",
      },
    );
    store.emit(
      {
        key: "submitTx",
      },
      {
        data: {
          txHash: "0x7375626d69747465642d74780000000000000000",
        },
        message: "submitted",
        status: "success",
      },
    );

    expect(listener).toHaveBeenNthCalledWith(1, {
      status: "pending",
    });
    expect(listener).toHaveBeenNthCalledWith(2, {
      data: {
        txHash: "0x7375626d69747465642d74780000000000000000",
      },
      message: "submitted",
      status: "success",
    });
    expect(client.blockSubscription).not.toHaveBeenCalled();
    expect(client.eventsSubscription).not.toHaveBeenCalled();
    expect(client.transferSubscription).not.toHaveBeenCalled();

    unsubscribe();
    store.emit(
      {
        key: "submitTx",
      },
      {
        description: "not delivered",
        status: "error",
        title: "Error",
      },
    );

    expect(listener).toHaveBeenCalledTimes(2);
  });

  it("continues dispatching events when one listener fails and reports the listener error", () => {
    const globalErrorHandler = vi.fn();
    const listenerErrorHandler = vi.fn();
    const listenerError = new Error("listener failed");
    const client = createSubscriptionClient();
    const store = subscriptionsStore(client as never, {
      onError: globalErrorHandler,
    });
    const failingListener = vi.fn(() => {
      throw listenerError;
    });
    const healthyListener = vi.fn();
    const params = {
      sinceBlockHeight: 21,
      username: "alice",
    };

    store.subscribe("transfer", {
      listener: failingListener,
      onError: listenerErrorHandler,
      params,
    });
    store.subscribe("transfer", {
      listener: healthyListener,
      params,
    });

    store.emit(
      {
        key: "transfer",
        params,
      },
      {
        transfers: [{ hash: "0x6572726f722d69736f6c6174696f6e0000000000" }],
      } as never,
    );

    expect(failingListener).toHaveBeenCalledWith({
      transfers: [{ hash: "0x6572726f722d69736f6c6174696f6e0000000000" }],
    });
    expect(healthyListener).toHaveBeenCalledWith({
      transfers: [{ hash: "0x6572726f722d69736f6c6174696f6e0000000000" }],
    });
    expect(globalErrorHandler).toHaveBeenCalledWith(listenerError);
    expect(listenerErrorHandler).toHaveBeenCalledWith(listenerError);
  });

  it("routes backend subscription errors to active listeners sharing the executor", () => {
    const executorUnsubscribe = vi.fn();
    const globalErrorHandler = vi.fn();
    const firstErrorHandler = vi.fn();
    const secondErrorHandler = vi.fn();
    let emitBackendError: ((error: unknown) => void) | undefined;
    const client = createSubscriptionClient();
    client.eventsSubscription.mockImplementation(({ error }) => {
      emitBackendError = error;
      return executorUnsubscribe;
    });
    const store = subscriptionsStore(client as never, {
      onError: globalErrorHandler,
    });
    const params = {
      filter: [{ type: "transfer" }],
      sinceBlockHeight: 55,
    };

    const unsubscribeFirst = store.subscribe("events", {
      listener: vi.fn(),
      onError: firstErrorHandler,
      params,
    });
    const unsubscribeSecond = store.subscribe("events", {
      listener: vi.fn(),
      onError: secondErrorHandler,
      params,
    });

    expect(client.eventsSubscription).toHaveBeenCalledOnce();

    const firstBackendError = new Error("events subscription failed");
    emitBackendError?.(firstBackendError);

    expect(globalErrorHandler).toHaveBeenCalledWith(firstBackendError);
    expect(firstErrorHandler).toHaveBeenCalledWith(firstBackendError);
    expect(secondErrorHandler).toHaveBeenCalledWith(firstBackendError);

    unsubscribeFirst();
    expect(executorUnsubscribe).not.toHaveBeenCalled();

    vi.clearAllMocks();

    const secondBackendError = new Error("events subscription restarted");
    emitBackendError?.(secondBackendError);

    expect(globalErrorHandler).toHaveBeenCalledWith(secondBackendError);
    expect(firstErrorHandler).not.toHaveBeenCalled();
    expect(secondErrorHandler).toHaveBeenCalledWith(secondBackendError);

    unsubscribeSecond();
    expect(executorUnsubscribe).toHaveBeenCalledOnce();
  });

  it("cleans up failed backend subscription setup before retrying", () => {
    const setupError = new Error("events subscription setup failed");
    const executorUnsubscribe = vi.fn();
    let emitEvents: ((event: { events: unknown[] }) => void) | undefined;
    const client = createSubscriptionClient();
    client.eventsSubscription
      .mockImplementationOnce(() => {
        throw setupError;
      })
      .mockImplementationOnce(({ next }) => {
        emitEvents = next;
        return executorUnsubscribe;
      });
    const store = subscriptionsStore(client as never);
    const staleListener = vi.fn();
    const staleErrorHandler = vi.fn();
    const retryListener = vi.fn();
    const retryErrorHandler = vi.fn();
    const params = {
      filter: [{ type: "transfer" }],
      sinceBlockHeight: 88,
    };

    expect(() =>
      store.subscribe("events", {
        listener: staleListener,
        onError: staleErrorHandler,
        params,
      }),
    ).toThrow(setupError);

    const unsubscribeRetry = store.subscribe("events", {
      listener: retryListener,
      onError: retryErrorHandler,
      params,
    });

    expect(client.eventsSubscription).toHaveBeenCalledTimes(2);

    emitEvents?.({
      events: [{ id: "retry-event" }],
    });

    expect(staleListener).not.toHaveBeenCalled();
    expect(staleErrorHandler).not.toHaveBeenCalled();
    expect(retryListener).toHaveBeenCalledWith([{ id: "retry-event" }]);

    unsubscribeRetry();
    expect(executorUnsubscribe).toHaveBeenCalledOnce();
  });
});
