import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseMediaQueryFactory } from "./mocks/applets-kit";

import { TradeHeader } from "../src/components/dex/components/TradeHeader";

const tradeHeaderMocks = vi.hoisted(() => ({
  isLg: true,
  onChangePair: vi.fn(),
  pair: {
    base: {
      decimals: 8,
      denom: "bridge/btc",
      logoURI: "/images/coins/bitcoin.svg",
      name: "Bitcoin",
      symbol: "BTC",
      type: "native",
    },
    id: "perp/btcusd",
    logoURI: "/images/coins/bitcoin.svg",
    name: "Bitcoin",
    quote: {
      decimals: 6,
      denom: "usd",
      logoURI: "/images/coins/usd.svg",
      name: "US Dollar",
      symbol: "USD",
      type: "native",
    },
    ticker: "BTCUSD",
    type: "crypto",
  },
  useAllPerpsPairStats: vi.fn(),
  useCurrentPrice: vi.fn(),
  useOraclePrices: vi.fn(),
  usePerpsPairStatsByPairId: vi.fn(),
  usePerpsPairParam: vi.fn(),
  usePerpsPairState: vi.fn(),
  usePerpsParam: vi.fn(),
  usePerpsState: vi.fn(),
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
      transition: _transition,
      ...props
    }: React.HTMLAttributes<HTMLDivElement> & {
      animate?: unknown;
      exit?: unknown;
      initial?: unknown;
      layout?: unknown;
      layoutId?: string;
      transition?: unknown;
    }) => <div {...props}>{children}</div>,
  },
}));

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
    }),
    useCountdown: () => ({
      hours: "01",
      minutes: "02",
      seconds: "03",
    }),
  };
});

vi.mock("../src/components/dex/components/ProTrade", () => ({
  useProTrade: () => ({
    onChangeTicker: tradeHeaderMocks.onChangePair,
    pair: tradeHeaderMocks.pair,
  }),
}));

vi.mock("../src/components/dex/components/SearchToken", () => ({
  SearchToken: ({
    onChangePair,
  }: {
    pair: typeof tradeHeaderMocks.pair;
    onChangePair: (row: { pair: typeof tradeHeaderMocks.pair }) => void;
  }) => (
    <button
      onClick={() =>
        onChangePair({
          pair: { ...tradeHeaderMocks.pair, id: "perp/ethusd", ticker: "ETHUSD" },
        })
      }
      type="button"
    >
      BTCUSD
    </button>
  ),
}));

vi.mock("@left-curve/store", () => ({
  useAllPerpsPairStats: tradeHeaderMocks.useAllPerpsPairStats,
  useCurrentPrice: tradeHeaderMocks.useCurrentPrice,
  useOraclePrices: tradeHeaderMocks.useOraclePrices,
  usePerpsPairStatsByPairId: tradeHeaderMocks.usePerpsPairStatsByPairId,
  usePerpsPairParam: tradeHeaderMocks.usePerpsPairParam,
  usePerpsPairState: tradeHeaderMocks.usePerpsPairState,
  usePerpsParam: tradeHeaderMocks.usePerpsParam,
  usePerpsState: tradeHeaderMocks.usePerpsState,
}));

describe("DEX trade header", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    tradeHeaderMocks.isLg = true;
    setAppletsKitUseMediaQueryFactory(() => ({
      isLg: tradeHeaderMocks.isLg,
    }));
    tradeHeaderMocks.useAllPerpsPairStats.mockImplementation(
      (selector: (state: { perpsPairStatsByPairId: Record<string, unknown> }) => unknown) =>
        selector({
          perpsPairStatsByPairId: {
            "perp/btcusd": {
              currentPrice: "101",
              price24HAgo: "100",
              priceChange24H: "1",
              volume24H: "12345",
            },
          },
        }),
    );
    tradeHeaderMocks.usePerpsPairStatsByPairId.mockReturnValue({
      currentPrice: "101",
      pairId: "perp/btcusd",
      price24HAgo: "100",
      priceChange24H: "1",
      volume24H: "12345",
    });
    tradeHeaderMocks.useCurrentPrice.mockReturnValue({
      currentPrice: "101",
      previousPrice: "99",
    });
    tradeHeaderMocks.useOraclePrices.mockImplementation(
      (selector: (state: { prices: Record<string, { humanizedPrice: string }> }) => unknown) =>
        selector({
          prices: {
            "bridge/btc": {
              humanizedPrice: "100.5",
            },
          },
        }),
    );
    tradeHeaderMocks.usePerpsPairParam.mockReturnValue({
      data: {
        maxAbsOi: "3",
      },
    });
    tradeHeaderMocks.usePerpsPairState.mockImplementation(
      (
        selector: (state: {
          pairState: {
            fundingRate: string;
            longOi: string;
            shortOi: string;
          };
        }) => unknown,
      ) =>
        selector({
          pairState: {
            fundingRate: "0.00024",
            longOi: "3",
            shortOi: "2",
          },
        }),
    );
    tradeHeaderMocks.usePerpsParam.mockReturnValue({
      data: {
        fundingPeriod: 3600,
      },
    });
    tradeHeaderMocks.usePerpsState.mockImplementation(
      (selector: (state: { state: { lastFundingTime: string } }) => unknown) =>
        selector({
          state: {
            lastFundingTime: "1717243200",
          },
        }),
    );

    class ResizeObserverMock {
      disconnect = vi.fn();
      observe = vi.fn();
    }

    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: ResizeObserverMock,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders backend-fed price, stats, open-interest, and funding metrics", async () => {
    const { container } = render(<TradeHeader />);

    expect(screen.getByText("BTCUSD")).toBeInTheDocument();
    expect(screen.getByText("Perp")).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.spot.lastPrice"]())).toBeInTheDocument();
    expect(screen.getByText("101")).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.spot.oraclePrice"]())).toBeInTheDocument();
    expect(
      screen.getByText((_, node) => node?.tagName === "P" && node.textContent === "100.5"),
    ).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.spot.24hChange"]())).toBeInTheDocument();
    expect(
      screen.getByText((_, node) => node?.tagName === "P" && node.textContent === "+1 / +1%"),
    ).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.spot.volume"]())).toBeInTheDocument();
    expect(
      screen.getByText((_, node) => node?.tagName === "P" && node.textContent === "$12,345"),
    ).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.spot.openInterest"]())).toBeInTheDocument();
    expect(
      screen.getByText((_, node) => node?.tagName === "P" && node.textContent === "$505.00"),
    ).toBeInTheDocument();
    const oiLimitIcon = container.querySelector("svg.text-status-fail");
    expect(oiLimitIcon).not.toBeNull();
    fireEvent.mouseEnter(oiLimitIcon!.parentElement!);
    expect(await screen.findByText(m["dex.protrade.spot.oiLimitReached"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.spot.funding"]())).toBeInTheDocument();
    expect(
      screen.getByText((_, node) => node?.tagName === "SPAN" && node.textContent === "0.001%"),
    ).toBeInTheDocument();
    expect(screen.getByText("01:02:03")).toBeInTheDocument();
    const fundingRate = screen.getByText(
      (_, node) => node?.tagName === "SPAN" && node.textContent === "0.001%",
    );
    fireEvent.mouseEnter(fundingRate.parentElement!);
    await waitFor(() => expect(screen.getByText("Annualized: 8.76%")).toBeInTheDocument());
  });

  it("routes selected search-token rows as tickers", () => {
    render(<TradeHeader />);

    fireEvent.click(screen.getByRole("button", { name: "BTCUSD" }));

    expect(tradeHeaderMocks.onChangePair).toHaveBeenCalledWith("ETHUSD");
  });

  it("collapses and expands metrics on mobile", () => {
    tradeHeaderMocks.isLg = false;

    const { container } = render(<TradeHeader />);

    expect(screen.queryByText(m["dex.protrade.spot.lastPrice"]())).not.toBeInTheDocument();

    const toggle = container.querySelector(".cursor-pointer");
    expect(toggle).not.toBeNull();
    fireEvent.click(toggle!);

    expect(screen.getByText(m["dex.protrade.spot.lastPrice"]())).toBeInTheDocument();
  });
});
