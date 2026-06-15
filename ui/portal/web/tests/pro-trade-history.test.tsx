import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { resetAppletsKitMocks, setAppletsKitUseAppFactory } from "./mocks/applets-kit";

import { Modals } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { MarketPair } from "@left-curve/foundation/market-pair";

import { ProTrade } from "../src/components/dex/components/ProTrade";

const accountAddress = "0x7472616465720000000000000000000000000000";

const proTradeHistoryMocks = vi.hoisted(() => ({
  onChangeAction: vi.fn(),
  onChangeOrderType: vi.fn(),
  onChangeTicker: vi.fn(),
  showModal: vi.fn(),
  toastDismiss: vi.fn(),
  toastError: vi.fn(),
}));

vi.mock("framer-motion", async () => {
  const React = await import("react");

  const motionComponent = <Element extends HTMLElement>(tag: string) =>
    React.forwardRef<Element, Record<string, unknown> & { children?: React.ReactNode }>(
      (
        {
          animate: _animate,
          children,
          exit: _exit,
          initial: _initial,
          layout: _layout,
          layoutId: _layoutId,
          transition: _transition,
          ...props
        },
        ref,
      ) => React.createElement(tag, { ...props, ref }, children),
    );

  return {
    AnimatePresence: ({ children }: { children: React.ReactNode }) =>
      React.createElement(React.Fragment, null, children),
    motion: {
      div: motionComponent<HTMLDivElement>("div"),
      li: motionComponent<HTMLLIElement>("li"),
      ul: motionComponent<HTMLUListElement>("ul"),
    },
  };
});

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
      showModal: proTradeHistoryMocks.showModal,
      toast: {
        dismiss: proTradeHistoryMocks.toastDismiss,
        error: proTradeHistoryMocks.toastError,
      },
    }),
  };
});

vi.mock("../src/app.sentry", () => ({
  reportStoreError: vi.fn(),
}));

vi.mock("../src/components/dex/components/OrderBookOverview", () => ({
  OrderBookOverview: () => <div data-testid="order-book-overview" />,
}));

vi.mock("../src/components/dex/components/TradeButtons", () => ({
  TradeButtons: () => <div data-testid="trade-buttons" />,
}));

vi.mock("../src/components/dex/components/TradeHeader", () => ({
  TradeHeader: () => <div data-testid="trade-header" />,
}));

vi.mock("../src/components/dex/components/TradeHistory", () => ({
  PerpsTradeHistory: () => <div data-testid="perps-trade-history" />,
}));

vi.mock("../src/components/dex/components/TradeMenu", () => ({
  TradeMenu: () => <div data-testid="trade-menu" />,
}));

vi.mock("@left-curve/store", () => {
  const coins = {
    byDenom: {
      "bridge/btc": {
        decimals: 8,
        denom: "bridge/btc",
        logoURI: "/btc.svg",
        name: "Bitcoin",
        symbol: "BTC",
        type: "native",
      },
      "bridge/eth": {
        decimals: 18,
        denom: "bridge/eth",
        logoURI: "/eth.svg",
        name: "Ethereum",
        symbol: "ETH",
        type: "native",
      },
      usd: {
        decimals: 6,
        denom: "usd",
        logoURI: "/usd.svg",
        name: "USD",
        symbol: "USD",
        type: "native",
      },
    },
  };

  const userState = {
    positions: {
      "perp/btcusd": {
        conditionalOrderAbove: {
          orderId: "tp-btc",
          triggerPrice: "53000",
        },
        entryPrice: "50000",
        size: "2",
      },
      "perp/ethusd": {
        entryPrice: "2000",
        size: "-3",
      },
    },
  };

  const orders = {
    "order-btc": {
      limitPrice: "51000",
      pairId: "perp/btcusd",
      reduceOnly: false,
      size: "0.5",
    },
    "order-eth": {
      limitPrice: "1900",
      pairId: "perp/ethusd",
      reduceOnly: true,
      size: "-2",
    },
  };

  const stats = {
    "perp/btcusd": {
      currentPrice: "51000",
    },
    "perp/ethusd": {
      currentPrice: "1800",
    },
  };

  const extendedPositions = {
    "perp/btcusd": {
      liquidationPrice: "42000",
    },
    "perp/ethusd": {
      liquidationPrice: "2600",
    },
  };

  return {
    useAccount: () => ({
      account: {
        address: accountAddress,
      },
    }),
    useAllPerpsPairStats: (
      selector: (state: {
        error: null;
        perpsPairStatsByPairId: typeof stats;
        status: "ready";
      }) => unknown,
    ) =>
      selector({
        error: null,
        perpsPairStatsByPairId: stats,
        status: "ready",
      }),
    useConfig: () => ({
      coins,
    }),
    useCurrentPrice: () => ({
      currentPrice: "51000",
    }),
    useLivePerpsTrades: (selector: (state: { error: null; status: "ready" }) => unknown) =>
      selector({ error: null, status: "ready" }),
    useOraclePrices: (selector: (state: { error: null; status: "ready" }) => unknown) =>
      selector({ error: null, status: "ready" }),
    usePerpsOrdersByUser: (
      selector: (state: { error: null; orders: typeof orders; status: "ready" }) => unknown,
    ) => selector({ error: null, orders, status: "ready" }),
    usePerpsPairState: (selector: (state: { error: null; status: "ready" }) => unknown) =>
      selector({ error: null, status: "ready" }),
    usePerpsState: (selector: (state: { error: null; status: "ready" }) => unknown) =>
      selector({ error: null, status: "ready" }),
    usePerpsUserState: (
      selector: (state: { error: null; status: "ready"; userState: typeof userState }) => unknown,
    ) => selector({ error: null, status: "ready", userState }),
    usePerpsUserStateExtended: (
      selector: (state: {
        equity: string;
        error: null;
        positions: typeof extendedPositions;
        status: "ready";
      }) => unknown,
    ) =>
      selector({
        equity: "4200",
        error: null,
        positions: extendedPositions,
        status: "ready",
      }),
  };
});

const originalResizeObserver = globalThis.ResizeObserver;

function renderProTradeHistory() {
  return render(
    <ProTrade
      action="buy"
      onChangeAction={proTradeHistoryMocks.onChangeAction}
      onChangeOrderType={proTradeHistoryMocks.onChangeOrderType}
      onChangeTicker={proTradeHistoryMocks.onChangeTicker}
      orderType="market"
      pair={MarketPair.fromTicker("BTCUSD")}
    >
      <ProTrade.History />
    </ProTrade>,
  );
}

function rowForText(text: string) {
  const row = screen.getByText(text).closest("tr");
  if (!(row instanceof HTMLTableRowElement)) throw new Error(`Expected row for ${text}`);
  return row;
}

function clickTab(label: string) {
  const tabButton = screen.getByText(label).closest("button");
  if (!(tabButton instanceof HTMLButtonElement))
    throw new Error(`Expected tab button for ${label}`);
  fireEvent.click(tabButton);
}

describe("ProTrade history", () => {
  beforeAll(() => {
    globalThis.ResizeObserver = class {
      disconnect() {}
      observe() {}
      unobserve() {}
    } as typeof ResizeObserver;
  });

  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseAppFactory(() => ({
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
      showModal: proTradeHistoryMocks.showModal,
      toast: {
        dismiss: proTradeHistoryMocks.toastDismiss,
        error: proTradeHistoryMocks.toastError,
      },
    }));
    document.title = "Dango";
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  afterAll(() => {
    if (originalResizeObserver) globalThis.ResizeObserver = originalResizeObserver;
    else Reflect.deleteProperty(globalThis, "ResizeObserver");
  });

  it("maps backend positions into row actions with backend modal payloads", () => {
    renderProTradeHistory();

    const btcRow = rowForText("BTCUSD");
    fireEvent.click(btcRow);
    expect(proTradeHistoryMocks.onChangeTicker).toHaveBeenCalledWith("BTCUSD");

    fireEvent.click(within(btcRow).getByText(m["dex.protrade.positions.edit"]()));
    expect(proTradeHistoryMocks.showModal).toHaveBeenCalledWith(Modals.ProSwapEditedSL, {
      conditionalOrderAbove: {
        orderId: "tp-btc",
        triggerPrice: "53000",
      },
      conditionalOrderBelow: undefined,
      entryPrice: "50000",
      markPrice: "51000",
      pairId: "perp/btcusd",
      size: "2",
      symbol: "BTC",
    });

    proTradeHistoryMocks.onChangeTicker.mockClear();
    const pnlShareButton = within(btcRow)
      .getAllByRole("button")
      .find((button) => button.querySelector("svg") && !button.textContent?.trim());
    if (!(pnlShareButton instanceof HTMLButtonElement))
      throw new Error("Expected position PnL share button");

    fireEvent.click(pnlShareButton);
    expect(proTradeHistoryMocks.onChangeTicker).not.toHaveBeenCalled();
    expect(proTradeHistoryMocks.showModal).toHaveBeenCalledWith(Modals.PnlShare, {
      currentPrice: 51000,
      entryPrice: "50000",
      equity: "4200",
      mode: "position",
      pairId: "perp/btcusd",
      pnl: 2000,
      size: "2",
      symbol: "BTC",
    });

    fireEvent.click(
      within(btcRow).getByRole("button", { name: m["dex.protrade.positions.close"]() }),
    );
    expect(proTradeHistoryMocks.showModal).toHaveBeenCalledWith(Modals.PerpsClosePosition, {
      pairId: "perp/btcusd",
      pnl: 2000,
      size: "2",
    });
  });

  it("opens the add-TP/SL modal for positions without existing conditional orders", () => {
    renderProTradeHistory();

    const ethRow = rowForText("ETHUSD");
    fireEvent.click(within(ethRow).getByText(m["dex.protrade.positions.edit"]()));

    expect(proTradeHistoryMocks.showModal).toHaveBeenCalledWith(Modals.ProSwapEditTPSL, {
      conditionalOrderAbove: undefined,
      conditionalOrderBelow: undefined,
      entryPrice: "2000",
      markPrice: "1800",
      pairId: "perp/ethusd",
      size: "-3",
      symbol: "ETH",
    });
  });

  it("filters backend open orders by active pair and opens cancel modals with order ids", () => {
    renderProTradeHistory();

    clickTab(m["dex.protrade.openOrders"]());

    expect(rowForText("BTCUSD")).toBeInTheDocument();
    expect(rowForText("ETHUSD")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["common.cancelAll"]() }));
    expect(proTradeHistoryMocks.showModal).toHaveBeenCalledWith(Modals.PerpsCloseAll, {});

    const btcOrderRow = rowForText("BTCUSD");
    fireEvent.click(within(btcOrderRow).getByRole("button", { name: m["common.cancel"]() }));
    expect(proTradeHistoryMocks.showModal).toHaveBeenCalledWith(Modals.PerpsCloseOrder, {
      orderId: "order-btc",
    });

    fireEvent.click(
      screen.getByRole("checkbox", { name: m["dex.protrade.orders.showAllPairs"]() }),
    );

    expect(rowForText("BTCUSD")).toBeInTheDocument();
    expect(screen.queryByText("ETHUSD")).not.toBeInTheDocument();
  });
});
