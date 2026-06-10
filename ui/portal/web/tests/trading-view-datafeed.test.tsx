import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CandleInterval } from "@left-curve/types";
import { createPerpsDataFeed } from "../datafeed.config";

import type { PerpsCandle } from "@left-curve/types";

const candleTemplate = {
  close: "0",
  high: "0",
  interval: CandleInterval.OneMinute,
  low: "0",
  maxBlockHeight: 1,
  minBlockHeight: 1,
  open: "0",
  pairId: "perp/ethusd",
  timeStart: "2026-06-09T00:00:00.000Z",
  timeStartUnix: 0,
  volume: "0",
  volumeUsd: "0",
} satisfies PerpsCandle;

function candle(overrides: Partial<PerpsCandle>): PerpsCandle {
  return {
    ...candleTemplate,
    ...overrides,
  };
}

function createDatafeedFixture() {
  const client = {
    queryPerpsCandles: vi.fn(),
  };
  const queryClient = {
    fetchQuery: vi.fn(async ({ queryFn }: { queryFn: () => Promise<unknown> }) => queryFn()),
  };
  const subscriptions = {
    subscribe: vi.fn(),
  };

  return {
    client,
    datafeed: createPerpsDataFeed({
      client: client as never,
      queryClient: queryClient as never,
      subscriptions: subscriptions as never,
    }),
    queryClient,
    subscriptions,
  };
}

describe("TradingView perps datafeed", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("fetches backend candles with normalized pair ids, interval enums, and ascending TradingView bars", async () => {
    const { client, datafeed, queryClient } = createDatafeedFixture();
    client.queryPerpsCandles.mockResolvedValue({
      nodes: [
        candle({
          close: "2050",
          high: "2060",
          low: "1990",
          open: "2000",
          timeStartUnix: 1_759_449_600_000,
          volumeUsd: "125.5",
        }),
        candle({
          close: "1980",
          high: "2010",
          low: "1975",
          open: "1995",
          timeStartUnix: 1_759_446_000_000,
          volumeUsd: "80",
        }),
      ],
    });
    const onHistory = vi.fn();
    const onError = vi.fn();

    datafeed.getBars(
      { name: "ETH-USD" } as never,
      "15" as never,
      { to: 1_759_453_200 } as never,
      onHistory,
      onError,
    );
    await vi.runAllTimersAsync();

    const earlierThan = new Date(1_759_453_200_000);
    expect(queryClient.fetchQuery).toHaveBeenCalledWith({
      queryKey: ["perpsCandles", "perp/ethusd", earlierThan, CandleInterval.FifteenMinutes],
      queryFn: expect.any(Function),
    });
    expect(client.queryPerpsCandles).toHaveBeenCalledWith({
      earlierThan: earlierThan.toJSON(),
      interval: CandleInterval.FifteenMinutes,
      pairId: "perp/ethusd",
    });
    expect(onHistory).toHaveBeenCalledWith(
      [
        {
          close: 1980,
          high: 2010,
          low: 1975,
          open: 1995,
          time: 1_759_446_000_000,
          volume: 80,
        },
        {
          close: 2050,
          high: 2060,
          low: 1990,
          open: 2000,
          time: 1_759_449_600_000,
          volume: 125.5,
        },
      ],
      { noData: false },
    );
    expect(onError).not.toHaveBeenCalled();
  });

  it("returns noData for empty responses and surfaces backend errors to TradingView", async () => {
    const { client, datafeed } = createDatafeedFixture();
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const onHistory = vi.fn();
    const onError = vi.fn();

    client.queryPerpsCandles.mockResolvedValueOnce({ nodes: [] });
    datafeed.getBars(
      { name: "BTC-USD" } as never,
      "1D" as never,
      { to: 1 } as never,
      onHistory,
      onError,
    );
    await vi.runAllTimersAsync();

    expect(client.queryPerpsCandles).toHaveBeenCalledWith({
      earlierThan: new Date(1_000).toJSON(),
      interval: CandleInterval.OneDay,
      pairId: "perp/btcusd",
    });
    expect(onHistory).toHaveBeenCalledWith([], { noData: true });
    expect(onError).not.toHaveBeenCalled();

    client.queryPerpsCandles.mockRejectedValueOnce(new Error("indexer unavailable"));
    datafeed.getBars(
      { name: "BTC-USD" } as never,
      "1D" as never,
      { to: 2 } as never,
      onHistory,
      onError,
    );
    await vi.runAllTimersAsync();

    expect(onError).toHaveBeenCalledWith("indexer unavailable");
    consoleError.mockRestore();
  });

  it("streams only the active backend candle subscription and tears down the previous one", () => {
    const { datafeed, subscriptions } = createDatafeedFixture();
    const firstUnsubscribe = vi.fn();
    const secondUnsubscribe = vi.fn();
    let secondListener: ((event: { perpsCandles: PerpsCandle[] }) => void) | undefined;
    subscriptions.subscribe
      .mockImplementationOnce(() => {
        return firstUnsubscribe;
      })
      .mockImplementationOnce((_key, { listener }) => {
        secondListener = listener;
        return secondUnsubscribe;
      });
    const onRealtime = vi.fn();

    datafeed.subscribeBars({ name: "ETH-USD" } as never, "5" as never, onRealtime, "eth-sub");
    expect(subscriptions.subscribe).toHaveBeenNthCalledWith(1, "perpsCandles", {
      listener: expect.any(Function),
      params: {
        interval: CandleInterval.FiveMinutes,
        pairId: "perp/ethusd",
      },
    });

    datafeed.subscribeBars({ name: "BTC-USD" } as never, "1S" as never, onRealtime, "btc-sub");
    expect(firstUnsubscribe).toHaveBeenCalledOnce();
    expect(subscriptions.subscribe).toHaveBeenNthCalledWith(2, "perpsCandles", {
      listener: expect.any(Function),
      params: {
        interval: CandleInterval.OneSecond,
        pairId: "perp/btcusd",
      },
    });

    secondListener?.({ perpsCandles: [] });
    expect(onRealtime).not.toHaveBeenCalled();

    secondListener?.({
      perpsCandles: [
        candle({
          close: "65000",
          high: "65100",
          low: "64000",
          open: "64500",
          pairId: "perp/btcusd",
          timeStartUnix: 1_759_453_200_000,
          volumeUsd: "5000",
        }),
      ],
    });

    expect(onRealtime).toHaveBeenCalledWith({
      close: 65000,
      high: 65100,
      low: 64000,
      open: 64500,
      time: 1_759_453_200_000,
      volume: 5000,
    });
    expect(secondUnsubscribe).not.toHaveBeenCalled();
  });
});
