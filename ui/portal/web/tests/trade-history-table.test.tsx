import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { PerpsEvent } from "@left-curve/types";

import {
  resetAppletsKitMocks,
  setAppletsKitUseAppFactory,
  setAppletsKitUseMediaQuery,
} from "./mocks/applets-kit";

import { Modals } from "@left-curve/applets-kit";

import { PerpsTradeHistory } from "../src/components/dex/components/TradeHistory";

const tradeHistoryTableMocks = vi.hoisted(() => ({
  fetchNextPage: vi.fn(),
  navigate: vi.fn(),
  showModal: vi.fn(),
  usePerpsTradeHistory: vi.fn(),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => tradeHistoryTableMocks.navigate,
}));

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count, estimateSize }: { count: number; estimateSize: () => number }) => ({
    getTotalSize: () => count * estimateSize(),
    getVirtualItems: () =>
      Array.from({ length: count }, (_, index) => ({
        index,
        key: `row-${index}`,
        size: estimateSize(),
        start: index * estimateSize(),
      })),
  }),
}));

vi.mock("@left-curve/foundation", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@left-curve/foundation")>()),
  useApp: () => ({
    settings: {
      formatNumberOptions: {
        language: "en-US",
        mask: 1,
      },
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    account: {
      address: "0x7472616465720000000000000000000000000000",
    },
  }),
  usePublicClient: () => ({
    queryPerpsEvents: vi.fn(),
  }),
}));

vi.mock("../src/components/dex/components/TradeHistory/usePerpsTradeHistory", () => ({
  usePerpsTradeHistory: tradeHistoryTableMocks.usePerpsTradeHistory,
}));

const orderFilledEvent: PerpsEvent = {
  blockHeight: 321,
  createdAt: "2026-06-08T10:00:00.000Z",
  data: {
    closing_size: "0",
    fee: "12.5",
    fill_price: "60000",
    fill_size: "0.5",
    is_maker: true,
    opening_size: "0.5",
    order_id: "order-1",
    pair_id: "perp/btcusd",
    realized_funding: "1.25",
    realized_pnl: "40",
    user: "0x7472616465720000000000000000000000000000",
  },
  eventType: "order_filled",
  idx: 1,
  pairId: "perp/btcusd",
  txHash: "0x6f72646572000000000000000000000000000000000000000000000000000000",
  userAddr: "0x7472616465720000000000000000000000000000",
};

const deleveragedEvent: PerpsEvent = {
  blockHeight: 322,
  createdAt: "2026-06-08T10:01:00.000Z",
  data: {
    closing_size: "-2",
    fill_price: "2.5",
    pair_id: "perp/xauusd",
    realized_funding: "-0.1",
    realized_pnl: "-3",
    user: "0x7472616465720000000000000000000000000000",
  },
  eventType: "deleveraged",
  idx: 2,
  pairId: "perp/xauusd",
  txHash: "0x64656c65766572616765640000000000000000000000000000000000000000",
  userAddr: "0x7472616465720000000000000000000000000000",
};

const historicalOrderFilledEvent: PerpsEvent = {
  ...orderFilledEvent,
  blockHeight: 299,
  createdAt: "2026-04-20T10:00:00.000Z",
  data: {
    ...orderFilledEvent.data,
    fee: "0",
    realized_funding: "7.25",
    realized_pnl: "0",
  },
  txHash: "0x686973746f726963616c000000000000000000000000000000000000000000",
};

function getByTextContent(text: string) {
  return screen.getByText((_content, element) => {
    if (!element?.textContent?.includes(text)) return false;
    return Array.from(element.children).every((child) => !child.textContent?.includes(text));
  });
}

function expectTextStyled(text: string, className: string) {
  expect(screen.getByText(text).closest(`.${className}`)).not.toBeNull();
}

describe("PerpsTradeHistory table", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseAppFactory(() => ({
      showModal: tradeHistoryTableMocks.showModal,
    }));
    setAppletsKitUseMediaQuery({
      isMd: true,
    });
    vi.stubGlobal(
      "ResizeObserver",
      class ResizeObserver {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
    tradeHistoryTableMocks.usePerpsTradeHistory.mockReturnValue({
      fetchNextPage: tradeHistoryTableMocks.fetchNextPage,
      hasNextPage: true,
      isFetchingNextPage: false,
      isLoading: false,
      nodes: [orderFilledEvent, deleveragedEvent],
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("renders normalized backend trade rows, shares fill PnL, and routes row clicks to blocks", async () => {
    render(<PerpsTradeHistory />);

    const [queryRange] = tradeHistoryTableMocks.usePerpsTradeHistory.mock.calls.at(-1)!;
    expect(queryRange.earlierThan).toBeUndefined();
    expect(queryRange.laterThan).toEqual(expect.stringMatching(/^\d{4}-\d{2}-\d{2}T.*Z$/));
    expect(Date.parse(queryRange.laterThan)).not.toBeNaN();
    await waitFor(() => {
      expect(tradeHistoryTableMocks.fetchNextPage).toHaveBeenCalledOnce();
    });

    expect(screen.getByText("BTCUSD")).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.tradeHistory.eventType.trade"]())).toBeInTheDocument();
    expectTextStyled(m["dex.protrade.tradeHistory.side.buy"](), "text-green-500");
    expect(getByTextContent("0.5 BTC")).toBeInTheDocument();
    expect(getByTextContent("$30,000")).toBeInTheDocument();
    expect(getByTextContent("+40").closest(".text-green-500")).not.toBeNull();
    expect(screen.getByText(m["dex.protrade.tradeHistory.maker"]())).toBeInTheDocument();

    expect(screen.getByText("XAUUSD")).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.tradeHistory.eventType.adl"]())).toBeInTheDocument();
    expectTextStyled(m["dex.protrade.tradeHistory.side.sell"](), "text-red-500");
    expect(getByTextContent("2 XAU")).toBeInTheDocument();
    expect(getByTextContent("$5")).toBeInTheDocument();
    expect(getByTextContent("-3").closest(".text-red-500")).not.toBeNull();

    const firstTradeRow = screen.getByText("BTCUSD").closest('[role="button"]');
    expect(firstTradeRow).not.toBeNull();
    const shareButton = firstTradeRow!.querySelector("button");
    expect(shareButton).not.toBeNull();

    fireEvent.click(shareButton!);

    expect(tradeHistoryTableMocks.showModal).toHaveBeenCalledWith(Modals.PnlShare, {
      createdAt: orderFilledEvent.createdAt,
      fillPrice: "60000",
      mode: "fill",
      pairId: "perp/btcusd",
      realizedPnl: "40",
      size: "0.5",
      symbol: "BTC",
    });
    expect(tradeHistoryTableMocks.navigate).not.toHaveBeenCalled();

    fireEvent.click(screen.getByText("XAUUSD").closest('[role="button"]')!);

    expect(tradeHistoryTableMocks.navigate).toHaveBeenCalledWith({
      params: {
        block: "322",
      },
      to: "/block/$block",
    });
  });

  it("marks historical backend fields unavailable before funding and maker/taker cutoffs", () => {
    tradeHistoryTableMocks.usePerpsTradeHistory.mockReturnValue({
      fetchNextPage: tradeHistoryTableMocks.fetchNextPage,
      hasNextPage: false,
      isFetchingNextPage: false,
      isLoading: false,
      nodes: [historicalOrderFilledEvent],
    });

    render(<PerpsTradeHistory />);

    expect(screen.getByText("BTCUSD")).toBeInTheDocument();
    expect(screen.getAllByText("N/A")).toHaveLength(2);
    expect(screen.queryByText("7.25")).not.toBeInTheDocument();
    expect(screen.queryByText(m["dex.protrade.tradeHistory.maker"]())).not.toBeInTheDocument();
  });

  it("shows range controls and export by default", () => {
    tradeHistoryTableMocks.usePerpsTradeHistory.mockReturnValue({
      fetchNextPage: tradeHistoryTableMocks.fetchNextPage,
      hasNextPage: false,
      isFetchingNextPage: false,
      isLoading: false,
      nodes: [],
    });

    render(<PerpsTradeHistory />);

    const [queryRange] = tradeHistoryTableMocks.usePerpsTradeHistory.mock.calls.at(-1)!;
    expect(queryRange.earlierThan).toBeUndefined();
    expect(queryRange.laterThan).toEqual(expect.stringMatching(/^\d{4}-\d{2}-\d{2}T.*Z$/));
    expect(Date.parse(queryRange.laterThan)).not.toBeNaN();
    expect(
      screen.getByRole("button", { name: m["dex.protrade.tradeHistory.preset.1d"]() }),
    ).toHaveClass("bg-surface-primary-blue");
    expect(
      screen.getByRole("button", { name: m["dex.protrade.tradeHistory.exportCsv"]() }),
    ).toBeDisabled();
  });
});
