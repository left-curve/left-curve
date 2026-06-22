import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  buildPerpsTradeHistoryCsv,
  type PerpsCsvHeaders,
} from "../src/components/dex/components/TradeHistory/exportCsv";
import {
  EXPORT_PAGE_DELAY_MS,
  EXPORT_PAGE_SIZE,
  ExportCsvButton,
  fetchAllPerpsTradeHistory,
} from "../src/components/dex/components/TradeHistory/ExportCsvButton";

import type { PublicClient } from "@left-curve/sdk";
import type { GraphqlQueryResult, PageInfo, PerpsEvent } from "@left-curve/types";

const tradeHistoryExportMocks = vi.hoisted(() => ({
  downloadCsv: vi.fn(),
  queryPerpsEvents: vi.fn(),
  useAccount: vi.fn(),
  usePublicClient: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: tradeHistoryExportMocks.useAccount,
  usePublicClient: tradeHistoryExportMocks.usePublicClient,
}));

vi.mock("../src/components/dex/components/TradeHistory/exportCsv", async (importOriginal) => {
  const actual =
    await importOriginal<
      typeof import("../src/components/dex/components/TradeHistory/exportCsv")
    >();

  return {
    ...actual,
    downloadCsv: tradeHistoryExportMocks.downloadCsv,
    tradeHistoryCsvFilename: () => "trade-history-perps-test.csv",
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
  txHash: "0x6f72646572000000000000000000000000000000000000000000000000000000",
  userAddr: "0x7472616465720000000000000000000000000000",
};

const secondOrderFilledEvent: PerpsEvent = {
  ...orderFilledEvent,
  blockHeight: 101,
  createdAt: "2026-06-08T10:01:00.000Z",
  idx: 2,
  txHash: "0x7365636f6e640000000000000000000000000000000000000000000000000000",
};

const queryRange = {
  earlierThan: "2026-06-09T00:00:00.000Z",
  laterThan: "2026-06-01T00:00:00.000Z",
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

function csvHeaders(overrides: Partial<PerpsCsvHeaders> = {}): PerpsCsvHeaders {
  return {
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
    ...overrides,
  };
}

describe("perps trade history CSV export button", () => {
  beforeEach(() => {
    tradeHistoryExportMocks.useAccount.mockReturnValue({
      account: {
        address: "0x7472616465720000000000000000000000000000",
      },
    });
    tradeHistoryExportMocks.usePublicClient.mockReturnValue({
      queryPerpsEvents: tradeHistoryExportMocks.queryPerpsEvents,
    });
    tradeHistoryExportMocks.queryPerpsEvents.mockResolvedValue(
      createGraphqlPage([orderFilledEvent], {
        endCursor: "first-end",
        hasNextPage: false,
        hasPreviousPage: false,
        startCursor: "first-start",
      }),
    );
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("disables export until both an account and trade-history events are available", () => {
    const { rerender } = render(<ExportCsvButton events={[]} queryRange={queryRange} />);

    expect(
      screen.getByRole("button", { name: m["dex.protrade.tradeHistory.exportCsv"]() }),
    ).toBeDisabled();

    tradeHistoryExportMocks.useAccount.mockReturnValue({ account: undefined });
    rerender(<ExportCsvButton events={[orderFilledEvent]} queryRange={queryRange} />);

    expect(
      screen.getByRole("button", { name: m["dex.protrade.tradeHistory.exportCsv"]() }),
    ).toBeDisabled();
    expect(tradeHistoryExportMocks.downloadCsv).not.toHaveBeenCalled();
  });

  it("builds and downloads a CSV with normalized backend event fields", async () => {
    render(<ExportCsvButton events={[orderFilledEvent]} queryRange={queryRange} />);

    fireEvent.click(
      screen.getByRole("button", { name: m["dex.protrade.tradeHistory.exportCsv"]() }),
    );

    await waitFor(() => expect(tradeHistoryExportMocks.downloadCsv).toHaveBeenCalledOnce());
    expect(tradeHistoryExportMocks.queryPerpsEvents).toHaveBeenCalledWith({
      after: undefined,
      earlierThan: queryRange.earlierThan,
      first: EXPORT_PAGE_SIZE,
      laterThan: queryRange.laterThan,
      sortBy: "BLOCK_HEIGHT_DESC",
      userAddr: "0x7472616465720000000000000000000000000000",
    });

    const [filename, csv] = tradeHistoryExportMocks.downloadCsv.mock.calls[0];
    expect(filename).toBe("trade-history-perps-test.csv");
    expect(csv).toContain(m["dex.protrade.tradeHistory.pair"]());
    expect(csv).toContain("BTCUSD");
    expect(csv).toContain("0.5 BTC");
    expect(csv).toContain("10000");
    expect(csv).toContain("2026-06-08T10:00:00.000Z");
  });

  it("shows an inline spinner while export is running", async () => {
    let resolvePage!: (page: GraphqlQueryResult<PerpsEvent>) => void;
    const pagePromise = new Promise<GraphqlQueryResult<PerpsEvent>>((resolve) => {
      resolvePage = resolve;
    });
    tradeHistoryExportMocks.queryPerpsEvents.mockReturnValueOnce(pagePromise);

    render(<ExportCsvButton events={[orderFilledEvent]} queryRange={queryRange} />);

    const button = screen.getByRole("button", {
      name: m["dex.protrade.tradeHistory.exportCsv"](),
    });
    fireEvent.click(button);

    expect(button).toBeDisabled();
    expect(screen.getByLabelText("Exporting CSV, 0 trades")).toBeInTheDocument();

    await act(async () => {
      resolvePage(
        createGraphqlPage([orderFilledEvent], {
          endCursor: "first-end",
          hasNextPage: false,
          hasPreviousPage: false,
          startCursor: "first-start",
        }),
      );
      await pagePromise;
    });

    await waitFor(() => expect(tradeHistoryExportMocks.downloadCsv).toHaveBeenCalledOnce());
    expect(screen.queryByLabelText(/Exporting CSV/)).not.toBeInTheDocument();
  });

  it("fetches every export page with cursor pagination and throttles between pages", async () => {
    const queryPerpsEvents = vi
      .fn()
      .mockResolvedValueOnce(
        createGraphqlPage([orderFilledEvent], {
          endCursor: "first-end",
          hasNextPage: true,
          hasPreviousPage: false,
          startCursor: "first-start",
        }),
      )
      .mockResolvedValueOnce(
        createGraphqlPage([secondOrderFilledEvent], {
          endCursor: "second-end",
          hasNextPage: false,
          hasPreviousPage: true,
          startCursor: "second-start",
        }),
      );
    const onProgress = vi.fn();
    const wait = vi.fn().mockResolvedValue(undefined);

    const events = await fetchAllPerpsTradeHistory({
      address: "0x7472616465720000000000000000000000000000",
      client: { queryPerpsEvents } as unknown as Pick<PublicClient, "queryPerpsEvents">,
      onProgress,
      queryRange,
      wait,
    });

    expect(events).toEqual([orderFilledEvent, secondOrderFilledEvent]);
    expect(queryPerpsEvents).toHaveBeenNthCalledWith(1, {
      after: undefined,
      earlierThan: queryRange.earlierThan,
      first: EXPORT_PAGE_SIZE,
      laterThan: queryRange.laterThan,
      sortBy: "BLOCK_HEIGHT_DESC",
      userAddr: "0x7472616465720000000000000000000000000000",
    });
    expect(queryPerpsEvents).toHaveBeenNthCalledWith(2, {
      after: "first-end",
      earlierThan: queryRange.earlierThan,
      first: EXPORT_PAGE_SIZE,
      laterThan: queryRange.laterThan,
      sortBy: "BLOCK_HEIGHT_DESC",
      userAddr: "0x7472616465720000000000000000000000000000",
    });
    expect(wait).toHaveBeenCalledOnce();
    expect(wait).toHaveBeenCalledWith(EXPORT_PAGE_DELAY_MS);
    expect(onProgress.mock.calls).toEqual([[1], [2]]);
  });

  it("normalizes liquidation and ADL backend events into export rows", () => {
    const liquidatedEvent: PerpsEvent = {
      blockHeight: 101,
      createdAt: "2026-06-08T11:00:00.000Z",
      data: {
        adl_price: "25000",
        adl_realized_funding: "4",
        adl_realized_pnl: "-100",
        adl_size: "1.2",
        pair_id: "perp/ethusd",
        user: "0x6c69717569646174656400000000000000000000",
      },
      eventType: "liquidated",
      idx: 2,
      pairId: "perp/ethusd",
      txHash: "0x6c69710000000000000000000000000000000000000000000000000000000000",
      userAddr: "0x6c69717569646174656400000000000000000000",
    };
    const deleveragedEvent: PerpsEvent = {
      blockHeight: 102,
      createdAt: "2026-06-08T12:00:00.000Z",
      data: {
        closing_size: "-0.25",
        fill_price: "20000",
        pair_id: "perp/btcusd",
        realized_funding: null,
        realized_pnl: "12.4",
        user: "0x61646c0000000000000000000000000000000000",
      },
      eventType: "deleveraged",
      idx: 3,
      pairId: "perp/btcusd",
      txHash: "0x61646c00000000000000000000000000000000000000000000000000000000",
      userAddr: "0x61646c0000000000000000000000000000000000",
    };

    const rows = buildPerpsTradeHistoryCsv([liquidatedEvent, deleveragedEvent], csvHeaders()).split(
      "\n",
    );

    expect(rows[1].split(",")).toEqual([
      "ETHUSD",
      m["dex.protrade.tradeHistory.eventType.liquidation"](),
      m["dex.protrade.tradeHistory.side.buy"](),
      "1.2 ETH",
      "30000",
      "25000",
      "-100",
      "4",
      "",
      "",
      "2026-06-08T11:00:00.000Z",
    ]);
    expect(rows[2].split(",")).toEqual([
      "BTCUSD",
      m["dex.protrade.tradeHistory.eventType.adl"](),
      m["dex.protrade.tradeHistory.side.sell"](),
      "0.25 BTC",
      "5000",
      "20000",
      "12.4",
      "",
      "",
      "",
      "2026-06-08T12:00:00.000Z",
    ]);
  });

  it("escapes localized CSV headers that contain punctuation", () => {
    const csv = buildPerpsTradeHistoryCsv(
      [orderFilledEvent],
      csvHeaders({
        direction: "Direction\nSide",
        pair: "Pair, localized",
        type: 'Type "label"',
      }),
    );

    expect(csv).toContain('"Pair, localized","Type ""label""","Direction\nSide"');
    expect(csv).toContain("BTCUSD");
  });
});
