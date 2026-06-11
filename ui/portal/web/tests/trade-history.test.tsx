import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  buildPerpsTradeHistoryCsv,
  tradeHistoryCsvFilename,
} from "../src/components/dex/components/TradeHistory/exportCsv";
import { normalizePerpsEvent } from "../src/components/dex/helpers/normalizePerpsEvent";
import {
  getMakerTakerLabel,
  getPerpsEventLabel,
  getSideLabel,
} from "../src/components/dex/helpers/perpsEventLabels";
import { usePerpsTradeHistory } from "../src/components/dex/components/TradeHistory/usePerpsTradeHistory";
import { useTradeHistoryFilter } from "../src/components/dex/components/TradeHistory/useTradeHistoryFilter";
import { createQueryClientWrapper } from "./utils/query-client";

import type { GraphqlQueryResult, PageInfo, PerpsEvent } from "@left-curve/types";

const storeMocks = vi.hoisted(() => ({
  queryPerpsEvents: vi.fn(),
  useAccount: vi.fn(),
  usePublicClient: vi.fn(),
}));

vi.mock("@left-curve/store", async () => {
  const { useInfiniteGraphqlQuery } = await import(
    "../../../store/src/hooks/useInfiniteGraphqlQuery"
  );

  return {
    useAccount: storeMocks.useAccount,
    useInfiniteGraphqlQuery,
    usePublicClient: storeMocks.usePublicClient,
  };
});

const orderFilledEvent: PerpsEvent = {
  blockHeight: 100,
  createdAt: "2026-06-08T10:00:00.000Z",
  data: {
    closing_size: "0",
    fee: "12.5",
    fill_price: "20000",
    fill_size: "-0.5",
    is_maker: false,
    opening_size: "-0.5",
    order_id: "order-1",
    pair_id: "perp/btcusd",
    realized_funding: null,
    realized_pnl: "33.25",
    user: "0x7472616465720000000000000000000000000000",
  },
  eventType: "order_filled",
  idx: 1,
  pairId: "perp/btcusd",
  txHash: "tx-order",
  userAddr: "0x7472616465720000000000000000000000000000",
};

const liquidatedEvent: PerpsEvent = {
  blockHeight: 101,
  createdAt: "2026-06-08T10:01:00.000Z",
  data: {
    adl_price: null,
    adl_realized_funding: "-2",
    adl_realized_pnl: "-100",
    adl_size: "1.25",
    pair_id: "perp/ethusd",
    user: "0x7472616465720000000000000000000000000000",
  },
  eventType: "liquidated",
  idx: 2,
  pairId: "perp/ethusd",
  txHash: "tx-liquidated",
  userAddr: "0x7472616465720000000000000000000000000000",
};

const deleveragedEvent: PerpsEvent = {
  blockHeight: 102,
  createdAt: "2026-06-08T10:02:00.000Z",
  data: {
    closing_size: "0.75",
    fill_price: "3100",
    pair_id: "perp/ethusd",
    realized_funding: null,
    realized_pnl: "0",
    user: "0x7472616465720000000000000000000000000000",
  },
  eventType: "deleveraged",
  idx: 3,
  pairId: "perp/ethusd",
  txHash: "tx-deleveraged",
  userAddr: "0x7472616465720000000000000000000000000000",
};

function createGraphqlPage(
  nodes: PerpsEvent[],
  pageInfo: PageInfo,
): GraphqlQueryResult<PerpsEvent> {
  return {
    edge: nodes.map((node) => ({
      cursor: `${node.txHash}-cursor`,
      node,
    })),
    nodes,
    pageInfo,
  };
}

const csvHeaders = {
  direction: m["dex.protrade.tradeHistory.direction"](),
  fees: m["dex.protrade.tradeHistory.fees"](),
  funding: m["dex.protrade.tradeHistory.funding"](),
  makerTaker: m["dex.protrade.tradeHistory.makerTaker"](),
  pair: m["dex.protrade.tradeHistory.pair"](),
  pnl: m["dex.protrade.tradeHistory.pnl"](),
  price: m["dex.protrade.history.price"](),
  size: "Size",
  time: m["dex.protrade.tradeHistory.time"](),
  tradeValue: m["dex.protrade.tradeHistory.tradeValue"](),
  type: m["dex.protrade.history.type"](),
};

describe("perps trade history helpers", () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("normalizes backend perps events into common trade-history fields", () => {
    expect(normalizePerpsEvent(orderFilledEvent)).toEqual({
      fee: "12.5",
      funding: undefined,
      isMaker: false,
      pnl: "33.25",
      price: "20000",
      size: "-0.5",
    });
    expect(normalizePerpsEvent(liquidatedEvent)).toEqual({
      fee: undefined,
      funding: "-2",
      isMaker: undefined,
      pnl: "-100",
      price: undefined,
      size: "1.25",
    });
    expect(normalizePerpsEvent(deleveragedEvent)).toEqual({
      fee: undefined,
      funding: undefined,
      isMaker: undefined,
      pnl: "0",
      price: "3100",
      size: "0.75",
    });
    expect(
      normalizePerpsEvent({
        ...orderFilledEvent,
        eventType: "unknown_event",
      } as PerpsEvent),
    ).toEqual({
      fee: undefined,
      funding: undefined,
      isMaker: undefined,
      pnl: undefined,
      price: undefined,
      size: undefined,
    });
  });

  it("builds CSV rows with normalized labels, signed direction, value, and escaping", () => {
    const csv = buildPerpsTradeHistoryCsv(
      [
        orderFilledEvent,
        {
          ...liquidatedEvent,
          createdAt: "2026-06-08T10:01:00,000Z",
        },
      ],
      {
        ...csvHeaders,
        type: `${csvHeaders.type} "Name"`,
      },
    );

    const escapedTypeHeader = `"${`${csvHeaders.type} "Name"`.replaceAll('"', '""')}"`;

    expect(csv.split("\n")).toEqual([
      [
        csvHeaders.pair,
        escapedTypeHeader,
        csvHeaders.direction,
        csvHeaders.size,
        csvHeaders.tradeValue,
        csvHeaders.price,
        csvHeaders.pnl,
        csvHeaders.funding,
        csvHeaders.fees,
        csvHeaders.makerTaker,
        csvHeaders.time,
      ].join(","),
      [
        "BTC/USD",
        getPerpsEventLabel("order_filled"),
        getSideLabel(true),
        "0.5 BTC",
        "10000",
        "20000",
        "33.25",
        "",
        "12.5",
        getMakerTakerLabel(false),
        "2026-06-08T10:00:00.000Z",
      ].join(","),
      [
        "ETH/USD",
        getPerpsEventLabel("liquidated"),
        getSideLabel(false),
        "1.25 ETH",
        "",
        "",
        "-100",
        "-2",
        "",
        "",
        '"2026-06-08T10:01:00,000Z"',
      ].join(","),
    ]);
  });

  it("uses the current calendar date in exported trade-history filenames", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-08T20:00:00.000Z"));

    expect(tradeHistoryCsvFilename()).toBe("trade-history-perps-2026-06-08.csv");
  });

  it("maps custom filter ranges to backend query boundaries", () => {
    const { result } = renderHook(() => useTradeHistoryFilter());
    const from = new Date("2026-05-01T00:00:00.000Z");
    const to = new Date("2026-06-01T00:00:00.000Z");

    act(() => {
      result.current.setCustomRange(from, to);
    });

    expect(result.current.filter).toEqual({
      from,
      preset: null,
      to,
    });
    expect(result.current.queryRange).toEqual({
      earlierThan: "2026-06-01T00:00:00.000Z",
      laterThan: "2026-05-01T00:00:00.000Z",
    });
  });

  it("maps preset filter ranges to later-than-only backend query boundaries", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-09T12:00:00.000Z"));

    const { result } = renderHook(() => useTradeHistoryFilter());

    act(() => {
      result.current.setPreset("1w");
    });

    expect(result.current.filter.preset).toBe("1w");
    expect(result.current.filter.from.toISOString()).toBe("2026-06-02T12:00:00.000Z");
    expect(result.current.filter.to.toISOString()).toBe("2026-06-09T12:00:00.000Z");
    expect(result.current.queryRange).toEqual({
      earlierThan: undefined,
      laterThan: "2026-06-02T12:00:00.000Z",
    });
  });
});

describe("usePerpsTradeHistory", () => {
  beforeEach(() => {
    storeMocks.useAccount.mockReturnValue({
      account: {
        address: "0x7472616465720000000000000000000000000000",
      },
    });
    storeMocks.usePublicClient.mockReturnValue({
      queryPerpsEvents: storeMocks.queryPerpsEvents,
    });
    storeMocks.queryPerpsEvents.mockImplementation(({ after }: { after?: string }) =>
      after
        ? Promise.resolve(
            createGraphqlPage([liquidatedEvent], {
              endCursor: "second-end",
              hasNextPage: false,
              hasPreviousPage: true,
              startCursor: "second-start",
            }),
          )
        : Promise.resolve(
            createGraphqlPage([orderFilledEvent], {
              endCursor: "first-end",
              hasNextPage: true,
              hasPreviousPage: false,
              startCursor: "first-start",
            }),
          ),
    );
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("queries perps events with account and filter bounds, then flattens fetched pages", async () => {
    const { result } = renderHook(
      () =>
        usePerpsTradeHistory({
          earlierThan: "2026-06-08T00:00:00.000Z",
          laterThan: "2026-06-01T00:00:00.000Z",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.nodes).toEqual([orderFilledEvent]));

    expect(storeMocks.queryPerpsEvents).toHaveBeenCalledWith({
      after: undefined,
      before: undefined,
      earlierThan: "2026-06-08T00:00:00.000Z",
      first: 30,
      last: undefined,
      laterThan: "2026-06-01T00:00:00.000Z",
      sortBy: "BLOCK_HEIGHT_DESC",
      userAddr: "0x7472616465720000000000000000000000000000",
    });
    expect(result.current.hasNextPage).toBe(true);

    act(() => {
      result.current.fetchNextPage();
    });

    await waitFor(() => expect(result.current.nodes).toEqual([orderFilledEvent, liquidatedEvent]));

    expect(storeMocks.queryPerpsEvents).toHaveBeenLastCalledWith({
      after: "first-end",
      before: undefined,
      earlierThan: "2026-06-08T00:00:00.000Z",
      first: 30,
      last: undefined,
      laterThan: "2026-06-01T00:00:00.000Z",
      sortBy: "BLOCK_HEIGHT_DESC",
      userAddr: "0x7472616465720000000000000000000000000000",
    });
  });

  it("refetches perps events when the active account address changes", async () => {
    const firstAddress = "0x7472616465720000000000000000000000000000";
    const secondAddress = "0x7365636f6e640000000000000000000000000000";
    const secondAccountEvent: PerpsEvent = {
      ...liquidatedEvent,
      data: {
        ...liquidatedEvent.data,
        user: secondAddress,
      },
      txHash: "tx-second-account",
      userAddr: secondAddress,
    };

    storeMocks.useAccount.mockReturnValue({
      account: {
        address: firstAddress,
      },
    });
    storeMocks.queryPerpsEvents.mockImplementation(({ userAddr }: { userAddr: string }) =>
      Promise.resolve(
        createGraphqlPage(userAddr === secondAddress ? [secondAccountEvent] : [orderFilledEvent], {
          endCursor: `${userAddr}-end`,
          hasNextPage: false,
          hasPreviousPage: false,
          startCursor: `${userAddr}-start`,
        }),
      ),
    );

    const { result, rerender } = renderHook(
      () =>
        usePerpsTradeHistory({
          earlierThan: undefined,
          laterThan: undefined,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.nodes).toEqual([orderFilledEvent]));
    expect(storeMocks.queryPerpsEvents).toHaveBeenLastCalledWith(
      expect.objectContaining({
        userAddr: firstAddress,
      }),
    );

    storeMocks.useAccount.mockReturnValue({
      account: {
        address: secondAddress,
      },
    });
    rerender();

    await waitFor(() => expect(result.current.nodes).toEqual([secondAccountEvent]));
    expect(storeMocks.queryPerpsEvents).toHaveBeenLastCalledWith(
      expect.objectContaining({
        userAddr: secondAddress,
      }),
    );
  });

  it("does not query trade history without an account", () => {
    storeMocks.useAccount.mockReturnValue({
      account: undefined,
    });

    const { result } = renderHook(
      () =>
        usePerpsTradeHistory({
          earlierThan: undefined,
          laterThan: "2026-06-01T00:00:00.000Z",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(result.current.nodes).toEqual([]);
    expect(result.current.hasNextPage).toBe(false);
    expect(storeMocks.queryPerpsEvents).not.toHaveBeenCalled();
  });
});
