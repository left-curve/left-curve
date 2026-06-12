import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { formatDate } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
  setAppletsKitUseMediaQueryFactory,
} from "./mocks/applets-kit";
import { OrderBookOverview } from "../src/components/dex/components/OrderBookOverview";

const orderBookMocks = vi.hoisted(() => ({
  accountAddress: "0x6f72646572626f6f6b0000000000000000000000",
  pair: {
    base: {
      decimals: 8,
      denom: "bridge/btc",
      logoURI: "/btc.svg",
      name: "Bitcoin",
      symbol: "BTC",
      type: "native",
    },
    id: "perp/btcusd",
    logoURI: "/btc.svg",
    name: "Bitcoin",
    quote: {
      decimals: 6,
      denom: "usd",
      logoURI: "/usd.svg",
      name: "US Dollar",
      symbol: "USD",
      type: "native",
    },
    ticker: "BTCUSD",
    type: "crypto",
  },
  depthSnapshot: {
    error: null as Error | null,
    liquidityDepth: null as {
      asks: Record<string, { notional: string; size: string }>;
      bids: Record<string, { notional: string; size: string }>;
    } | null,
    status: "ready" as "idle" | "connecting" | "ready" | "error",
  },
  displayMode: "base" as "base" | "quote",
  is3XlTall: false,
  isLg: true,
  liveTrades: [] as Array<{
    blockHeight: number;
    createdAt: string;
    fillPrice: string;
    fillSize: string;
    orderId: string;
    tradeIdx: number;
  }>,
  navigate: vi.fn(),
  setDisplayMode: vi.fn(),
  setValue: vi.fn(),
  userOrders: {} as Record<string, { limitPrice: string; pairId: string }>,
}));

vi.mock("@tanstack/react-router", () => ({
  useRouter: () => ({
    navigate: orderBookMocks.navigate,
  }),
}));

vi.mock("../src/components/dex/components/ProTrade", () => ({
  useProTrade: () => ({
    accountAddress: orderBookMocks.accountAddress,
    pair: orderBookMocks.pair,
  }),
}));

vi.mock("framer-motion", () => ({
  AnimatePresence: ({ children }: React.PropsWithChildren) => <>{children}</>,
  motion: {
    div: ({
      animate: _animate,
      children,
      exit: _exit,
      initial: _initial,
      layout: _layout,
      layoutId: _layoutId,
      layoutRoot: _layoutRoot,
      transition: _transition,
      ...props
    }: React.HTMLAttributes<HTMLDivElement> & {
      animate?: unknown;
      exit?: unknown;
      initial?: unknown;
      layout?: unknown;
      layoutId?: string;
      layoutRoot?: unknown;
      transition?: unknown;
    }) => <div {...props}>{children}</div>,
    ul: ({
      animate: _animate,
      children,
      exit: _exit,
      initial: _initial,
      transition: _transition,
      ...props
    }: React.HTMLAttributes<HTMLUListElement> & {
      animate?: unknown;
      exit?: unknown;
      initial?: unknown;
      transition?: unknown;
    }) => <ul {...props}>{children}</ul>,
  },
}));

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();
  const appState = {
    settings: {
      formatNumberOptions: {
        language: "en-US",
        mask: 1,
      },
      timeFormat: "HH:mm",
    },
  };

  return {
    ...actual,
    useApp: (selector?: (state: typeof appState) => unknown) =>
      typeof selector === "function" ? selector(appState) : appState,
  };
});

vi.mock("@left-curve/store", () => ({
  useAppConfig: () => ({
    data: {
      perpsPairs: {
        "perp/btcusd": {
          bucketSizes: ["0.5", "1"],
        },
      },
    },
  }),
  useCurrentPrice: () => ({
    currentPrice: "30000",
    previousPrice: "29900",
  }),
  useLivePerpsTrades: (
    selector: (state: { trades: typeof orderBookMocks.liveTrades }) => unknown,
  ) =>
    selector({
      trades: orderBookMocks.liveTrades,
    }),
  usePerpsLiquidityDepth: (
    selector: (state: typeof orderBookMocks.depthSnapshot) => unknown,
    _parameters: {
      bucketSize: string;
      pairId: string;
    },
  ) => selector(orderBookMocks.depthSnapshot),
  usePerpsOrdersByUser: (
    selector: (state: { orders: typeof orderBookMocks.userOrders }) => unknown,
    _parameters: {
      accountAddress?: string;
    },
  ) =>
    selector({
      orders: orderBookMocks.userOrders,
    }),
  useStorage: () => [orderBookMocks.displayMode, orderBookMocks.setDisplayMode],
}));

function renderOrderBook() {
  return render(
    <OrderBookOverview onSelectPrice={(price) => orderBookMocks.setValue("price", price)} />,
  );
}

function hasTextContent(text: string, tagName = "P") {
  return (_content: string, node: Element | null) =>
    node?.tagName === tagName && node.textContent === text;
}

describe("DEX order book overview", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
        timeFormat: "HH:mm",
      },
    });
    setAppletsKitUseMediaQueryFactory(() => ({
      is3XlTall: orderBookMocks.is3XlTall,
      isLg: orderBookMocks.isLg,
    }));
    class ResizeObserverMock {
      disconnect = vi.fn();
      observe = vi.fn();
      unobserve = vi.fn();
    }

    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: ResizeObserverMock,
    });
    orderBookMocks.accountAddress = "0x6f72646572626f6f6b0000000000000000000000";
    orderBookMocks.depthSnapshot = {
      error: null,
      liquidityDepth: {
        asks: {
          "30000.5": {
            notional: "45000.75",
            size: "1.5",
          },
          "30001.5": {
            notional: "15000.75",
            size: "0.5",
          },
        },
        bids: {
          "29998.5": {
            notional: "29998.5",
            size: "1",
          },
          "29999.5": {
            notional: "59999",
            size: "2",
          },
        },
      },
      status: "ready",
    };
    orderBookMocks.displayMode = "base";
    orderBookMocks.is3XlTall = false;
    orderBookMocks.isLg = true;
    orderBookMocks.liveTrades = [];
    orderBookMocks.userOrders = {
      "order-1": {
        limitPrice: "30000.5",
        pairId: "perp/btcusd",
      },
      "order-other-pair": {
        limitPrice: "40000.5",
        pairId: "perp/ethusd",
      },
    };
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders backend depth, marks user orders, and writes selected prices into the trade form", async () => {
    renderOrderBook();

    await screen.findByText(hasTextContent("29,999.5"));

    expect(screen.getByText(m["dex.protrade.spread"]())).toBeInTheDocument();
    expect(screen.getByText(hasTextContent("30,000", "SPAN"))).toBeInTheDocument();
    expect(screen.getAllByText(hasTextContent("2")).length).toBeGreaterThan(0);
    expect(screen.getAllByText(hasTextContent("1.5")).length).toBeGreaterThan(0);

    const userOrderPrice = screen.getByText(hasTextContent("30,000.5"));
    expect(userOrderPrice.closest("div")?.querySelector('[aria-hidden="true"]')).toBeTruthy();

    fireEvent.click(userOrderPrice);

    expect(orderBookMocks.setValue).toHaveBeenCalledWith("price", "30000.5");
  });

  it("uses quote display mode totals from backend notional values and persists mode changes", async () => {
    orderBookMocks.displayMode = "quote";

    renderOrderBook();

    expect(await screen.findAllByText(hasTextContent("45,001"))).toHaveLength(2);

    expect(screen.getByText(m["dex.protrade.history.size"]({ symbol: "USD" }))).toBeInTheDocument();
    expect(screen.getAllByText(hasTextContent("59,999"))).toHaveLength(2);

    fireEvent.click(screen.getByRole("button", { name: "USD" }));
    const baseOption = screen.getAllByText("BTC").find((node) => node.closest("li"));
    expect(baseOption).toBeDefined();
    fireEvent.click(baseOption!);

    expect(orderBookMocks.setDisplayMode).toHaveBeenCalledWith("base");
  });

  it("shows an unavailable state when the live depth resource errors", async () => {
    orderBookMocks.depthSnapshot = {
      error: new Error("depth stream disconnected"),
      liquidityDepth: null,
      status: "error",
    };

    renderOrderBook();

    expect(await screen.findByText("Order book unavailable")).toBeInTheDocument();
  });

  it("renders live trades and opens the matching block when a trade row is selected", async () => {
    orderBookMocks.liveTrades = [
      {
        blockHeight: 123,
        createdAt: "2026-06-08T10:00:00.000Z",
        fillPrice: "30010",
        fillSize: "-0.25",
        orderId: "trade-1",
        tradeIdx: 0,
      },
    ];

    renderOrderBook();

    fireEvent.click(await screen.findByRole("button", { name: "trades" }));

    await waitFor(() => expect(screen.getByText(hasTextContent("30,010"))).toBeInTheDocument());
    expect(screen.getByText(hasTextContent("0.25"))).toBeInTheDocument();
    expect(
      screen.getByText(formatDate("2026-06-08T10:00:00.000Z", "HH:mm:ss")),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText(hasTextContent("30,010")));

    expect(orderBookMocks.navigate).toHaveBeenCalledWith({
      to: "/block/123",
    });
  });

  it("routes live trades from backend block height zero to the block-zero page", async () => {
    orderBookMocks.liveTrades = [
      {
        blockHeight: 0,
        createdAt: "2026-06-08T10:00:00.000Z",
        fillPrice: "30020",
        fillSize: "0.1",
        orderId: "genesis-trade",
        tradeIdx: 0,
      },
    ];

    renderOrderBook();

    fireEvent.click(await screen.findByRole("button", { name: "trades" }));
    const tradePrice = await screen.findByText(hasTextContent("30,020"));

    fireEvent.click(tradePrice);

    expect(orderBookMocks.navigate).toHaveBeenCalledWith({
      to: "/block/0",
    });
  });
});
