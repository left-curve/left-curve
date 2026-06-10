import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetAppletsKitMocks, setAppletsKitUseMediaQueryFactory } from "./mocks/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { SearchToken } from "../src/components/dex/components/SearchToken";
import { SearchTokenTable } from "../src/components/dex/components/SearchTokenTable";

const searchTokenMocks = vi.hoisted(() => ({
  getPerpsAssetClass: vi.fn(),
  getPerpsPairIdFromPairId: vi.fn(),
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
  getPerpsAssetClass: searchTokenMocks.getPerpsAssetClass,
  getPerpsPairIdFromPairId: searchTokenMocks.getPerpsPairIdFromPairId,
  perpsMarginAsset: {
    decimals: 6,
    denom: "usd",
    logoURI: "/images/coins/usd.svg",
    name: "USD",
    symbol: "USD",
    type: "native",
  },
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

const goldCoin = {
  decimals: 6,
  denom: "oracle/gold",
  logoURI: "/gold.svg",
  name: "Gold",
  symbol: "XAU",
  type: "native",
};

const pairId = {
  baseDenom: btcCoin.denom,
  quoteDenom: "usd",
};

function createRow(
  overrides: Partial<React.ComponentProps<typeof SearchTokenTable>["data"][number]> = {},
) {
  return {
    baseCoin: btcCoin,
    boostMultiplier: undefined,
    isFavorite: false,
    pairId,
    pairKey: "BTC-USD",
    perpsPairId: "perp/btcusd",
    quoteCoin: {
      decimals: 6,
      denom: "usd",
      logoURI: "/images/coins/usd.svg",
      name: "USD",
      symbol: "USD",
      type: "native",
    },
    ...overrides,
  } as React.ComponentProps<typeof SearchTokenTable>["data"][number];
}

function rowForText(text: string, container: HTMLElement = document.body) {
  const row = within(container).getByText(text).closest("tr");
  if (!row) throw new Error(`Expected table row for ${text}`);
  return row;
}

function openDesktopSearchMenu() {
  const trigger = screen.getByRole("button", { name: /BTC-USD/ });
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
    searchTokenMocks.hasFavPair.mockImplementation((pairKey: string) => pairKey === "BTC-USD");
    searchTokenMocks.getPerpsAssetClass.mockImplementation((symbol: string) =>
      symbol === "XAU" ? "commodity" : "crypto",
    );
    searchTokenMocks.getPerpsPairIdFromPairId.mockImplementation(
      ({ baseDenom }: { baseDenom: string }) => {
        if (baseDenom === "bridge/btc") return "perp/btcusd";
        if (baseDenom === "bridge/eth") return "perp/ethusd";
        if (baseDenom === "oracle/gold") return "perp/xauusd";
        return "";
      },
    );
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
          "perp/missingusd": {},
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
          [goldCoin.denom]: goldCoin,
        },
        bySymbol: {
          BTC: btcCoin,
          ETH: ethCoin,
          XAU: goldCoin,
        },
      },
    });
    searchTokenMocks.useCurrentEpoch.mockReturnValue({
      currentEpoch: 9,
    });
    searchTokenMocks.useFavPairs.mockReturnValue({
      favPairs: ["BTC-USD"],
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
    const onChangePairId = vi.fn();

    render(<SearchToken pairId={pairId} onChangePairId={onChangePairId} />);

    const menu = openDesktopSearchMenu();
    expect(within(menu).getByText("BTC-USD")).toBeInTheDocument();
    expect(within(menu).getByText("ETH-USD")).toBeInTheDocument();
    expect(within(menu).getByText("XAU-USD")).toBeInTheDocument();
    expect(within(menu).queryByText("MISSING-USD")).not.toBeInTheDocument();
    expect(searchTokenMocks.useBoostedPairs).toHaveBeenCalledWith({
      currentEpoch: 9,
      pointsUrl: "https://points.test",
    });
    await expectBoostTooltip(rowForText("BTC-USD", menu), "2x points");

    fireEvent.change(screen.getByRole("textbox"), {
      target: {
        value: "eth",
      },
    });

    expect(within(menu).queryByText("BTC-USD")).not.toBeInTheDocument();
    expect(within(menu).getByText("ETH-USD")).toBeInTheDocument();

    fireEvent.pointerDown(rowForText("ETH-USD", menu), { button: 0 });

    expect(onChangePairId).toHaveBeenCalledWith(
      expect.objectContaining({
        baseCoin: ethCoin,
        pairId: {
          baseDenom: "bridge/eth",
          quoteDenom: "usd",
        },
        pairKey: "ETH-USD",
        perpsPairId: "perp/ethusd",
      }),
    );
  });

  it("filters favorites and asset-class tabs using store-backed state", () => {
    render(<SearchToken pairId={pairId} onChangePairId={vi.fn()} />);

    const menu = openDesktopSearchMenu();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["dex.protrade.searchPairTable.tabs.favorites"](),
      }),
    );

    expect(within(menu).getByText("BTC-USD")).toBeInTheDocument();
    expect(within(menu).queryByText("ETH-USD")).not.toBeInTheDocument();
    expect(within(menu).queryByText("XAU-USD")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["dex.protrade.searchPairTable.tabs.commodities"](),
      }),
    );

    expect(within(menu).getByText("XAU-USD")).toBeInTheDocument();
    expect(within(menu).queryByText("BTC-USD")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["dex.protrade.searchPairTable.tabs.crypto"](),
      }),
    );

    expect(within(menu).getByText("BTC-USD")).toBeInTheDocument();
    expect(within(menu).getByText("ETH-USD")).toBeInTheDocument();
    expect(within(menu).queryByText("XAU-USD")).not.toBeInTheDocument();
  });

  it("shows the favorites empty state only when the user has no favorite pairs", () => {
    searchTokenMocks.useFavPairs.mockReturnValue({
      favPairs: [],
      hasFavPair: vi.fn(() => false),
      toggleFavPair: searchTokenMocks.toggleFavPair,
    });

    render(<SearchToken pairId={pairId} onChangePairId={vi.fn()} />);
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
    const onChangePairId = vi.fn();

    render(
      <SearchTokenTable
        data={[
          createRow({
            boostMultiplier: "2.500000",
          }),
        ]}
        onChangePairId={onChangePairId}
      />,
    );

    expect(screen.getByText(m["dex.protrade.searchPairTable.name"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.searchPairTable.price"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.searchPairTable.24hChange"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.protrade.searchPairTable.volume"]())).toBeInTheDocument();
    const btcRow = rowForText("BTC-USD");
    expect(btcRow).toHaveTextContent("BTC-USD");
    expect(btcRow).toHaveTextContent("$65,000");
    expect(btcRow).toHaveTextContent("+4.2%");
    expect(btcRow).toHaveTextContent("$987,654");
    await expectBoostTooltip(btcRow, "2.5x points");

    fireEvent.click(screen.getByRole("button", { name: m["common.starToggle.remove"]() }));

    expect(searchTokenMocks.toggleFavPair).toHaveBeenCalledWith("BTC-USD");
    expect(onChangePairId).not.toHaveBeenCalled();

    fireEvent.pointerDown(btcRow, { button: 0 });

    expect(onChangePairId).toHaveBeenCalledWith(
      expect.objectContaining({
        pairKey: "BTC-USD",
      }),
    );
  });
});
