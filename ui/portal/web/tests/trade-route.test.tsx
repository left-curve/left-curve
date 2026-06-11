import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseHeaderHeight } from "./mocks/applets-kit";

const routeMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  params: {
    pairSymbols: "ETH-USD",
  },
  redirect: vi.fn((options: unknown) => ({
    options,
    type: "redirect",
  })),
  search: {} as {
    action?: "buy" | "sell";
    order_type?: "limit" | "market";
  },
}));

vi.mock("@tanstack/react-router", () => ({
  createFileRoute: (routePath: string) => (options: unknown) => ({
    options,
    routePath,
  }),
  createLazyFileRoute: (routePath: string) => (options: unknown) => ({
    options,
    routePath,
    useParams: () => routeMocks.params,
    useSearch: () => routeMocks.search,
  }),
  redirect: routeMocks.redirect,
  useNavigate: () => routeMocks.navigate,
}));

vi.mock("@left-curve/store", () => ({
  useConfig: () => ({
    coins: {
      bySymbol: {
        BTC: {
          denom: "bridge/btc",
        },
        ETH: {
          denom: "bridge/eth",
        },
        USDC: {
          denom: "bridge/usdc",
        },
      },
    },
  }),
}));

vi.mock("~/components/dex/components/ProTrade", () => {
  type ProTradeProps = {
    action: "buy" | "sell";
    children: React.ReactNode;
    onChangeAction: (action: "buy" | "sell") => void;
    onChangeOrderType: (orderType: "limit" | "market") => void;
    onChangePairId: (pairSymbols: string) => void;
    orderType: "limit" | "market";
    pairId: {
      baseDenom: string;
      quoteDenom: string;
    };
  };

  const ProTrade = ({
    action,
    children,
    onChangeAction,
    onChangeOrderType,
    onChangePairId,
    orderType,
    pairId,
  }: ProTradeProps) => (
    <section
      data-action={action}
      data-base-denom={pairId.baseDenom}
      data-order-type={orderType}
      data-quote-denom={pairId.quoteDenom}
      data-testid="pro-trade"
    >
      <button onClick={() => onChangePairId("BTC-USD")} type="button">
        change pair
      </button>
      <button onClick={() => onChangeAction(action === "buy" ? "sell" : "buy")} type="button">
        change action
      </button>
      <button
        onClick={() => onChangeOrderType(orderType === "market" ? "limit" : "market")}
        type="button"
      >
        change order type
      </button>
      {children}
    </section>
  );

  ProTrade.Header = () => <div data-testid="trade-header" />;
  ProTrade.Chart = () => <div data-testid="trade-chart" />;
  ProTrade.OrderBook = () => <div data-testid="trade-order-book" />;
  ProTrade.History = () => <div data-testid="trade-history" />;
  ProTrade.TradeMenu = () => <div data-testid="trade-menu" />;

  return {
    ProTrade,
  };
});

type TradeRoute = {
  options: {
    beforeLoad: (args: {
      context: {
        client: {
          getPerpsPairParam: ReturnType<typeof vi.fn>;
        };
        config: {
          chain: {
            name: string;
          };
        };
      };
      params: {
        pairSymbols: string;
      };
    }) => Promise<void>;
    validateSearch: {
      parse: (search: unknown) => {
        action: "buy" | "sell";
        order_type: "limit" | "market";
      };
    };
  };
};

type TradeIndexRoute = {
  options: {
    beforeLoad: (args: {
      context: {
        config: {
          chain: {
            name: string;
          };
        };
      };
    }) => Promise<void>;
  };
};

type TradeLazyRoute = {
  options: {
    component: React.ComponentType;
  };
};

let tradeRoutePromise: Promise<TradeRoute> | undefined;
let tradeLazyRoutePromise: Promise<TradeLazyRoute> | undefined;
let tradeIndexRoutePromise: Promise<TradeIndexRoute> | undefined;

async function loadTradeRoute() {
  routeMocks.redirect.mockClear();

  tradeRoutePromise ??= import("../src/pages/(app)/_app.trade.$pairSymbols").then(
    ({ Route }) => Route as unknown as TradeRoute,
  );
  return tradeRoutePromise;
}

async function loadTradeLazyRoute() {
  routeMocks.navigate.mockClear();

  tradeLazyRoutePromise ??= import("../src/pages/(app)/_app.trade.$pairSymbols.lazy").then(
    ({ Route }) => Route as unknown as TradeLazyRoute,
  );
  return tradeLazyRoutePromise;
}

async function loadTradeIndexRoute() {
  routeMocks.redirect.mockClear();

  tradeIndexRoutePromise ??= import("../src/pages/(app)/_app.trade.index").then(
    ({ Route }) => Route as unknown as TradeIndexRoute,
  );
  return tradeIndexRoutePromise;
}

function createRouteContext({
  chainName = "Mainnet",
  getPerpsPairParam = vi.fn().mockResolvedValue({ pairId: "perp/btcusd" }),
}: {
  chainName?: string;
  getPerpsPairParam?: ReturnType<typeof vi.fn>;
} = {}) {
  return {
    client: {
      getPerpsPairParam,
    },
    config: {
      chain: {
        name: chainName,
      },
    },
  };
}

function setLazyRouteState({
  pairSymbols = "ETH-USD",
  search = {},
}: {
  pairSymbols?: string;
  search?: typeof routeMocks.search;
} = {}) {
  routeMocks.params = {
    pairSymbols,
  };
  routeMocks.search = search;
}

async function expectRedirect(promise: Promise<unknown>) {
  await expect(promise).rejects.toMatchObject({
    type: "redirect",
  });
  return routeMocks.redirect.mock.results.at(-1)?.value;
}

describe("trade routes", () => {
  beforeAll(async () => {
    await Promise.all([loadTradeRoute(), loadTradeLazyRoute(), loadTradeIndexRoute()]);
  }, 20_000);

  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseHeaderHeight(88);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    setLazyRouteState();
  });

  it("keeps a backend-confirmed normalized pair on the trade pair route", async () => {
    const Route = await loadTradeRoute();
    const getPerpsPairParam = vi.fn().mockResolvedValue({ pairId: "perp/ethusd" });

    await expect(
      Route.options.beforeLoad({
        context: createRouteContext({ getPerpsPairParam }),
        params: {
          pairSymbols: "ETH-USD",
        },
      }),
    ).resolves.toBeUndefined();

    expect(getPerpsPairParam).toHaveBeenCalledWith({
      pairId: "perp/ethusd",
    });
    expect(routeMocks.redirect).not.toHaveBeenCalled();
  });

  it("normalizes pair symbols before asking the backend for pair params", async () => {
    const Route = await loadTradeRoute();
    const getPerpsPairParam = vi.fn();

    const redirect = await expectRedirect(
      Route.options.beforeLoad({
        context: createRouteContext({ getPerpsPairParam }),
        params: {
          pairSymbols: "eth",
        },
      }),
    );

    expect(redirect.options).toEqual({
      params: {
        pairSymbols: "ETH-USD",
      },
      to: "/trade/$pairSymbols",
    });
    expect(getPerpsPairParam).not.toHaveBeenCalled();
  });

  it("falls back to the chain default when the pair route is malformed", async () => {
    const Route = await loadTradeRoute();
    const getPerpsPairParam = vi.fn();

    const redirect = await expectRedirect(
      Route.options.beforeLoad({
        context: createRouteContext({ getPerpsPairParam }),
        params: {
          pairSymbols: "ETH-USD-EXTRA",
        },
      }),
    );

    expect(redirect.options).toEqual({
      params: {
        pairSymbols: "BTC-USD",
      },
      to: "/trade/$pairSymbols",
    });
    expect(getPerpsPairParam).not.toHaveBeenCalled();
  });

  it("falls back to the chain default when the backend has no perps pair params", async () => {
    const Route = await loadTradeRoute();
    const getPerpsPairParam = vi.fn().mockRejectedValue(new Error("missing pair"));

    const redirect = await expectRedirect(
      Route.options.beforeLoad({
        context: createRouteContext({
          chainName: "Devnet",
          getPerpsPairParam,
        }),
        params: {
          pairSymbols: "DOGE-USD",
        },
      }),
    );

    expect(getPerpsPairParam).toHaveBeenCalledWith({
      pairId: "perp/dogeusd",
    });
    expect(redirect.options).toEqual({
      params: {
        pairSymbols: "ETH-USD",
      },
      to: "/trade/$pairSymbols",
    });
  });

  it("defaults missing trade search params to market buys", async () => {
    const Route = await loadTradeRoute();

    expect(Route.options.validateSearch.parse({})).toEqual({
      action: "buy",
      order_type: "market",
    });
    expect(
      Route.options.validateSearch.parse({
        action: "sell",
        order_type: "limit",
      }),
    ).toEqual({
      action: "sell",
      order_type: "limit",
    });
    expect(() => Route.options.validateSearch.parse({ action: "close" })).toThrow();
  });

  it("redirects the trade index route to the chain default pair", async () => {
    const Route = await loadTradeIndexRoute();

    const redirect = await expectRedirect(
      Route.options.beforeLoad({
        context: {
          config: {
            chain: {
              name: "Devnet",
            },
          },
        },
      }),
    );

    expect(redirect.options).toEqual({
      params: {
        pairSymbols: "ETH-USD",
      },
      to: "/trade/$pairSymbols",
    });
  });

  it("maps route pair symbols and search params into the trade screen", async () => {
    setLazyRouteState({
      pairSymbols: "ETH-USDC",
      search: {
        action: "sell",
        order_type: "limit",
      },
    });
    const Route = await loadTradeLazyRoute();
    const Component = Route.options.component;

    render(<Component />);

    const proTrade = screen.getByTestId("pro-trade");
    expect(proTrade).toHaveAttribute("data-base-denom", "bridge/eth");
    expect(proTrade).toHaveAttribute("data-quote-denom", "bridge/usdc");
    expect(proTrade).toHaveAttribute("data-action", "sell");
    expect(proTrade).toHaveAttribute("data-order-type", "limit");
    expect(screen.getByTestId("trade-header")).toBeInTheDocument();
    expect(screen.getByTestId("trade-chart")).toBeInTheDocument();
    expect(screen.getByTestId("trade-order-book")).toBeInTheDocument();
    expect(screen.getByTestId("trade-history")).toBeInTheDocument();
    expect(screen.getByTestId("trade-menu")).toBeInTheDocument();
  });

  it("keeps trade screen navigation callbacks on the active pair route", async () => {
    setLazyRouteState({
      pairSymbols: "ETH-USD",
      search: {
        action: "sell",
        order_type: "limit",
      },
    });
    const Route = await loadTradeLazyRoute();
    const Component = Route.options.component;

    render(<Component />);

    fireEvent.click(screen.getByRole("button", { name: "change pair" }));
    expect(routeMocks.navigate).toHaveBeenLastCalledWith({
      params: {
        pairSymbols: "BTC-USD",
      },
      replace: true,
      to: "/trade/$pairSymbols",
    });

    fireEvent.click(screen.getByRole("button", { name: "change action" }));
    expect(routeMocks.navigate).toHaveBeenLastCalledWith({
      params: {
        pairSymbols: "ETH-USD",
      },
      replace: true,
      search: {
        action: "buy",
        order_type: "limit",
      },
      to: "/trade/$pairSymbols",
    });

    fireEvent.click(screen.getByRole("button", { name: "change order type" }));
    expect(routeMocks.navigate).toHaveBeenLastCalledWith({
      params: {
        pairSymbols: "ETH-USD",
      },
      replace: true,
      search: {
        action: "sell",
        order_type: "market",
      },
      to: "/trade/$pairSymbols",
    });
  });

  it("defaults the lazy trade screen to USD market buys when search params are absent", async () => {
    setLazyRouteState({
      pairSymbols: "ETH",
    });
    const Route = await loadTradeLazyRoute();
    const Component = Route.options.component;

    render(<Component />);

    const proTrade = screen.getByTestId("pro-trade");
    expect(proTrade).toHaveAttribute("data-base-denom", "bridge/eth");
    expect(proTrade).toHaveAttribute("data-quote-denom", "usd");
    expect(proTrade).toHaveAttribute("data-action", "buy");
    expect(proTrade).toHaveAttribute("data-order-type", "market");
  });
});
