import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetAppletsKitMocks, setAppletsKitUseMediaQueryFactory } from "./mocks/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { MarketPair } from "@left-curve/foundation/market-pair";

import type React from "react";

import { SearchToken } from "../src/components/dex/components/SearchToken";
import { SearchTokenTable } from "../src/components/dex/components/SearchTokenTable";

const searchTokenMocks = vi.hoisted(() => ({
  hasFavPair: vi.fn(),
  isLg: true,
  toggleFavPair: vi.fn(),
  useAllPerpsPairStats: vi.fn(),
  useAppConfig: vi.fn(),
  useBoostedPairs: vi.fn(),
  useConfig: vi.fn(),
  useCurrentEpoch: vi.fn(),
  useFavPairs: vi.fn(),
}));

class TestResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

vi.stubGlobal("ResizeObserver", TestResizeObserver);

vi.mock("react-modal-sheet", () => {
  const Sheet = ({
    children,
    isOpen,
  }: React.PropsWithChildren<{
    isOpen: boolean;
    onClose?: () => void;
    rootId?: string;
  }>) => (isOpen ? <div data-testid="sheet">{children}</div> : null);

  Sheet.Container = ({ children }: React.PropsWithChildren<{ className?: string }>) => (
    <div>{children}</div>
  );
  Sheet.Header = () => <header data-testid="sheet-header" />;
  Sheet.Content = ({ children }: React.PropsWithChildren) => <section>{children}</section>;
  Sheet.Backdrop = ({ onTap }: { onTap?: () => void }) => (
    <button onClick={onTap} type="button">
      backdrop
    </button>
  );

  return { Sheet };
});

vi.mock("@left-curve/foundation", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@left-curve/foundation")>()),
  useApp: () => ({
    settings: {
      formatNumberOptions: {
        currency: "USD",
        language: "en-US",
        mask: 1,
      },
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  useAllPerpsPairStats: searchTokenMocks.useAllPerpsPairStats,
  useAppConfig: searchTokenMocks.useAppConfig,
  useBoostedPairs: searchTokenMocks.useBoostedPairs,
  useConfig: searchTokenMocks.useConfig,
  useCurrentEpoch: searchTokenMocks.useCurrentEpoch,
  useFavPairs: searchTokenMocks.useFavPairs,
}));

const btcCoin = {
  decimals: 8,
  denom: "bridge/btc",
  logoURI: "/btc.svg",
  name: "Bitcoin",
  symbol: "BTC",
  type: "native",
};

const ethCoin = {
  decimals: 18,
  denom: "bridge/eth",
  logoURI: "/eth.svg",
  name: "Ethereum",
  symbol: "ETH",
  type: "native",
};

const pair = MarketPair.fromTicker("BTCUSD");

function createRow(
  overrides: Partial<React.ComponentProps<typeof SearchTokenTable>["data"][number]> = {},
) {
  return {
    boostMultiplier: undefined,
    isFavorite: false,
    pair,
    ...overrides,
  } as React.ComponentProps<typeof SearchTokenTable>["data"][number];
}

function rowForText(text: string, container: HTMLElement = document.body) {
  const row = within(container).getByText(text).closest("tr");
  if (!row) throw new Error(`Expected table row for ${text}`);
  return row;
}

function openDesktopSearchMenu() {
  const trigger = screen.getByRole("button", { name: /BTCUSD/ });
  fireEvent.click(trigger);

  const panelId = trigger.getAttribute("aria-controls");
  const panel = panelId ? document.getElementById(panelId) : null;
  expect(panel).not.toBeNull();

  return panel as HTMLElement;
}

async function expectBoostTooltip(row: HTMLElement, text: string) {
  const flameIcon = row.querySelector(".text-primitives-red-light-500");
  expect(flameIcon).not.toBeNull();

  const tooltipTrigger = flameIcon?.closest("div");
  expect(tooltipTrigger).not.toBeNull();

  fireEvent.mouseEnter(tooltipTrigger as Element);
  expect(await screen.findByRole("tooltip")).toHaveTextContent(text);
}

describe("DEX search token picker", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseMediaQueryFactory(() => ({
      isLg: searchTokenMocks.isLg,
    }));
    searchTokenMocks.isLg = true;
    searchTokenMocks.hasFavPair.mockImplementation((pairKey: string) => pairKey === "BTCUSD");
    searchTokenMocks.useAllPerpsPairStats.mockImplementation(
      (selector: (state: { perpsPairStatsByPairId: Record<string, unknown> }) => unknown) =>
        selector({
          perpsPairStatsByPairId: {
            "perp/btcusd": {
              currentPrice: "65000",
              priceChange24H: "4.2",
              volume24H: "987654",
            },
            "perp/ethusd": {
              currentPrice: "3100",
              priceChange24H: "-1.5",
              volume24H: "123456",
            },
          },
        }),
    );
    searchTokenMocks.useAppConfig.mockReturnValue({
      data: {
        perpsPairs: {
          "perp/btcusd": {},
          "perp/ethusd": {},
          "perp/xauusd": {},
        },
      },
    });
    searchTokenMocks.useBoostedPairs.mockReturnValue({
      boostByPairId: {
        "perp/btcusd": "2.000000",
      },
    });
    searchTokenMocks.useConfig.mockReturnValue({
      coins: {
        byDenom: {
          [btcCoin.denom]: btcCoin,
          [ethCoin.denom]: ethCoin,
        },
        bySymbol: {
          BTC: btcCoin,
          ETH: ethCoin,
        },
      },
    });
    searchTokenMocks.useCurrentEpoch.mockReturnValue({
      currentEpoch: 9,
    });
    searchTokenMocks.useFavPairs.mockReturnValue({
      favPairs: ["BTCUSD"],
      hasFavPair: searchTokenMocks.hasFavPair,
      toggleFavPair: searchTokenMocks.toggleFavPair,
    });

    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          pointsUrl: "https://points.test",
        },
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("builds rows from configured perps pairs, filters search text, and selects a pair", async () => {
    const onChangePair = vi.fn();

    render(<SearchToken pair={pair} onChangePair={onChangePair} />);

    const menu = openDesktopSearchMenu();
    expect(within(menu).getByText("BTCUSD")).toBeInTheDocument();
    expect(within(menu).getByText("ETHUSD")).toBeInTheDocument();
    expect(within(menu).getByText("XAUUSD")).toBeInTheDocument();
    expect(searchTokenMocks.useBoostedPairs).toHaveBeenCalledWith({
      currentEpoch: 9,
      pointsUrl: "https://points.test",
    });
    await expectBoostTooltip(rowForText("BTCUSD", menu), "2x points");

    fireEvent.change(screen.getByRole("textbox"), {
      target: {
        value: "eth",
      },
    });

    expect(within(menu).queryByText("BTCUSD")).not.toBeInTheDocument();
    expect(within(menu).getByText("ETHUSD")).toBeInTheDocument();

    fireEvent.pointerDown(rowForText("ETHUSD", menu), { button: 0 });

    expect(onChangePair).toHaveBeenCalledWith(
      expect.objectContaining({
        pair: MarketPair.fromTicker("ETHUSD"),
      }),
    );
  });

  it("filters favorites and asset-class tabs using store-backed state", () => {
    render(<SearchToken pair={pair} onChangePair={vi.fn()} />);

    const menu = openDesktopSearchMenu();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["dex.protrade.searchPairTable.tabs.favorites"](),
      }),
    );

    expect(within(menu).getByText("BTCUSD")).toBeInTheDocument();
    expect(within(menu).queryByText("ETHUSD")).not.toBeInTheDocument();
    expect(within(menu).queryByText("XAUUSD")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["dex.protrade.searchPairTable.tabs.commodities"](),
      }),
    );

    expect(within(menu).getByText("XAUUSD")).toBeInTheDocument();
    expect(within(menu).queryByText("BTCUSD")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["dex.protrade.searchPairTable.tabs.crypto"](),
      }),
    );

    expect(within(menu).getByText("BTCUSD")).toBeInTheDocument();
    expect(within(menu).getByText("ETHUSD")).toBeInTheDocument();
    expect(within(menu).queryByText("XAUUSD")).not.toBeInTheDocument();
  });

  it("shows the favorites empty state only when the user has no favorite pairs", () => {
    searchTokenMocks.useFavPairs.mockReturnValue({
      favPairs: [],
      hasFavPair: vi.fn(() => false),
      toggleFavPair: searchTokenMocks.toggleFavPair,
    });

    render(<SearchToken pair={pair} onChangePair={vi.fn()} />);
    openDesktopSearchMenu();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["dex.protrade.searchPairTable.tabs.favorites"](),
      }),
    );

    expect(
      screen.getByText(m["dex.protrade.searchPairTable.emptyFavorites"]()),
    ).toBeInTheDocument();
  });

  it("renders table stats and lets users toggle favorites without selecting the row", async () => {
    const onChangePair = vi.fn();

    render(
      <SearchTokenTable
        data={[
          createRow({
            boostMultiplier: "2.500000",
          }),
        ]}
        onChangePair={onChangePair}
      />,
    );

    expect(screen.getByText(m["dex.protrade.searchPairTable.name"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.searchPairTable.price"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.searchPairTable.24hChange"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.searchPairTable.volume"]())).toBeInTheDocument();
    const btcRow = rowForText("BTCUSD");
    expect(btcRow).toHaveTextContent("BTCUSD");
    expect(within(btcRow).getByText("Perp")).toBeInTheDocument();
    expect(btcRow).toHaveTextContent("$65,000");
    expect(btcRow).toHaveTextContent("+4.2%");
    expect(btcRow).toHaveTextContent("$987,654");
    await expectBoostTooltip(btcRow, "2.5x points");

    fireEvent.click(screen.getByRole("button", { name: m["common.starToggle.remove"]() }));

    expect(searchTokenMocks.toggleFavPair).toHaveBeenCalledWith("BTCUSD");
    expect(onChangePair).not.toHaveBeenCalled();

    fireEvent.pointerDown(btcRow, { button: 0 });

    expect(onChangePair).toHaveBeenCalledWith(
      expect.objectContaining({
        pair,
      }),
    );
  });
});
