import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseAppFactory } from "./mocks/applets-kit";

import { Modals, useInputs } from "@left-curve/applets-kit";

import { TradeMenu } from "../src/components/dex/components/TradeMenu";

const accountAddress = "0x7472616465720000000000000000000000000000";

const tradeMenuMocks = vi.hoisted(() => ({
  action: "buy" as "buy" | "sell",
  availToTrade: 1000,
  hideModal: vi.fn(),
  invalidateQueries: vi.fn(),
  isConnected: true,
  isGeoblocked: false,
  isPending: false,
  marginModeByPair: {} as Record<string, "cross" | "isolated">,
  maxSize: 1000,
  maxSlippage: "0.0125",
  mutateAsync: vi.fn(),
  onChangeAction: vi.fn(),
  onChangeOrderType: vi.fn(),
  orderType: "market" as "market" | "limit",
  setSidebarVisibility: vi.fn(),
  setTradeBarVisibility: vi.fn(),
  showModal: vi.fn(),
  submissionParameters: undefined as Record<string, unknown> | undefined,
  usePerpsSubmission: vi.fn(),
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

  const passthrough = {
    AnimatePresence: ({ children }: { children: React.ReactNode }) =>
      React.createElement(React.Fragment, null, children),
    motion: {
      div: motionComponent<HTMLDivElement>("div"),
      li: motionComponent<HTMLLIElement>("li"),
      ul: motionComponent<HTMLUListElement>("ul"),
    },
  };

  return passthrough;
});

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      changeSettings: vi.fn(),
      hideModal: tradeMenuMocks.hideModal,
      isQuestBannerVisible: true,
      isSearchBarVisible: false,
      isSidebarVisible: false,
      isTradeBarVisible: true,
      modal: { modal: undefined, props: {} },
      navigate: vi.fn(),
      setQuestBannerVisibility: vi.fn(),
      setSearchBarVisibility: vi.fn(),
      setSidebarVisibility: tradeMenuMocks.setSidebarVisibility,
      setTradeBarVisibility: tradeMenuMocks.setTradeBarVisibility,
      settings: {
        chart: "tradingview",
        dateFormat: "MM/dd/yyyy",
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
        isFirstVisit: false,
        showWelcome: false,
        timeFormat: "hh:mm a",
        timeZone: "local",
        useSessionKey: true,
      },
      showModal: tradeMenuMocks.showModal,
      subscriptions: {
        subscribe: vi.fn(),
      },
      toast: {
        error: vi.fn(),
        info: vi.fn(),
        promise: vi.fn(),
        success: vi.fn(),
        warning: vi.fn(),
      },
    }),
  };
});

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({
    invalidateQueries: tradeMenuMocks.invalidateQueries,
  }),
}));

vi.mock("~/components/foundation/hooks/useGeoblock", () => ({
  useGeoblock: () => tradeMenuMocks.isGeoblocked,
}));

vi.mock("../src/components/dex/components/ProTrade", () => {
  const pairId = {
    baseDenom: "bridge/btc",
    quoteDenom: "usd",
  };

  return {
    useProTrade: () => ({
      accountAddress,
      action: tradeMenuMocks.action,
      onChangeAction: tradeMenuMocks.onChangeAction,
      onChangeOrderType: tradeMenuMocks.onChangeOrderType,
      orderType: tradeMenuMocks.orderType,
      pairId,
      perpsPairId: "perp/btcusd",
    }),
  };
});

vi.mock("@left-curve/store", () => {
  const baseCoin = {
    decimals: 8,
    denom: "bridge/btc",
    logoURI: "/btc.svg",
    name: "Bitcoin",
    symbol: "BTC",
    type: "native",
  };
  const quoteCoin = {
    decimals: 6,
    denom: "usd",
    logoURI: "/usd.svg",
    name: "USD",
    symbol: "USD",
    type: "native",
  };
  const appConfig = {
    perpsPairs: {
      "perp/btcusd": {
        initialMarginRatio: "0.02",
        maintenanceMarginRatio: "0.01",
        minOrderSize: "10",
      },
    },
    perpsParam: {
      makerFeeRates: [],
      takerFeeRates: [],
    },
  };

  return {
    computeLiquidationPrice: vi.fn(() => null),
    perpsTradeSettingsStore: (
      selector: (state: {
        leverageByPair: Record<string, number>;
        marginModeByPair: Record<string, "cross" | "isolated">;
      }) => unknown,
    ) =>
      selector({
        leverageByPair: {},
        marginModeByPair: tradeMenuMocks.marginModeByPair,
      }),
    useAccount: () => ({
      account: tradeMenuMocks.isConnected ? { address: accountAddress } : undefined,
      isConnected: tradeMenuMocks.isConnected,
    }),
    useAllPerpsPairStats: (
      selector: (state: {
        perpsPairStatsByPairId: Record<string, { currentPrice: string }>;
      }) => unknown,
    ) =>
      selector({
        perpsPairStatsByPairId: {
          "perp/btcusd": {
            currentPrice: "50000",
          },
        },
      }),
    usePerpsPairStatsByPairId: ({ pairId }: { pairId: string }) => ({
      currentPrice: "50000",
      pairId,
      price24HAgo: "49000",
      priceChange24H: "2.04081632653061224489",
      volume24H: "123456",
    }),
    useAppConfig: () => ({
      data: appConfig,
    }),
    useConfig: () => ({
      coins: {
        bySymbol: {
          BTC: baseCoin,
        },
      },
      subscriptions: {
        subscribe: vi.fn(),
      },
    }),
    useFeeRateOverride: () => ({
      override: {
        makerFeeRate: "0.0005",
        takerFeeRate: "0.001",
      },
    }),
    usePerpsMaxSize: () => ({
      availToTrade: tradeMenuMocks.availToTrade,
      maxSize: tradeMenuMocks.maxSize,
    }),
    usePerpsSubmission: tradeMenuMocks.usePerpsSubmission,
    usePerpsUserState: (
      selector: (state: {
        userState: {
          margin: string;
          positions: Record<string, unknown>;
          reservedMargin: string;
        };
      }) => unknown,
    ) =>
      selector({
        userState: {
          margin: "2000",
          positions: {},
          reservedMargin: "0",
        },
      }),
    usePerpsUserStateExtended: (
      selector: (state: { equity: string; positions: Record<string, unknown> }) => unknown,
    ) =>
      selector({
        equity: "2000",
        positions: {},
      }),
    usePrices: () => ({
      getPrice: () => 50000,
    }),
    useStorage: () => [tradeMenuMocks.maxSlippage, vi.fn()],
    useTradeAccountCoins: () => ({
      baseCoin,
      quoteCoin,
    }),
    useTradePairCoins: () => ({
      baseCoin,
      quoteCoin,
    }),
    useVolume: () => ({
      volume: "0",
    }),
  };
});

const originalResizeObserver = globalThis.ResizeObserver;
const originalRequestAnimationFrame = globalThis.requestAnimationFrame;
const originalCancelAnimationFrame = globalThis.cancelAnimationFrame;

function TradeMenuHarness() {
  const controllers = useInputs({
    initialValues: {
      price: "",
      size: "",
      slPercent: "",
      slPrice: "",
      tpPercent: "",
      tpPrice: "",
    },
  });

  return <TradeMenu controllers={controllers} />;
}

function renderTradeMenu() {
  return render(<TradeMenuHarness />);
}

function getInput(name: string) {
  const input = document.querySelector<HTMLInputElement>(`input[name="${name}"]`);
  if (!input) throw new Error(`Expected ${name} input to be rendered`);
  return input;
}

function submitLabel() {
  return `${m["dex.protrade.perps.triggerAction"]({ action: tradeMenuMocks.action })} BTC`;
}

describe("DEX trade menu", () => {
  beforeAll(() => {
    globalThis.ResizeObserver = class {
      disconnect() {}
      observe() {}
      unobserve() {}
    } as typeof ResizeObserver;
    globalThis.requestAnimationFrame = ((callback: FrameRequestCallback) =>
      window.setTimeout(() => callback(Date.now()), 0)) as typeof requestAnimationFrame;
    globalThis.cancelAnimationFrame = ((id: number) =>
      window.clearTimeout(id)) as typeof cancelAnimationFrame;
  });

  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseAppFactory(() => ({
      hideModal: tradeMenuMocks.hideModal,
      isTradeBarVisible: true,
      setSidebarVisibility: tradeMenuMocks.setSidebarVisibility,
      setTradeBarVisibility: tradeMenuMocks.setTradeBarVisibility,
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
      showModal: tradeMenuMocks.showModal,
    }));
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 1280,
      writable: true,
    });
    Object.defineProperty(window, "innerHeight", {
      configurable: true,
      value: 900,
      writable: true,
    });

    tradeMenuMocks.action = "buy";
    tradeMenuMocks.availToTrade = 1000;
    tradeMenuMocks.isConnected = true;
    tradeMenuMocks.isGeoblocked = false;
    tradeMenuMocks.isPending = false;
    tradeMenuMocks.marginModeByPair = {};
    tradeMenuMocks.maxSize = 1000;
    tradeMenuMocks.maxSlippage = "0.0125";
    tradeMenuMocks.orderType = "market";
    tradeMenuMocks.submissionParameters = undefined;
    tradeMenuMocks.onChangeAction.mockImplementation((action: "buy" | "sell") => {
      tradeMenuMocks.action = action;
    });
    tradeMenuMocks.onChangeOrderType.mockImplementation((orderType: "market" | "limit") => {
      tradeMenuMocks.orderType = orderType;
    });
    tradeMenuMocks.usePerpsSubmission.mockImplementation(
      (parameters: Record<string, unknown> & { onSuccess?: () => void }) => {
        tradeMenuMocks.submissionParameters = parameters;
        tradeMenuMocks.mutateAsync = vi.fn(async () => {
          parameters.onSuccess?.();
        });

        return {
          isPending: tradeMenuMocks.isPending,
          mutateAsync: tradeMenuMocks.mutateAsync,
        };
      },
    );
  });

  afterEach(() => {
    cleanup();
    document.body.innerHTML = "";
    vi.clearAllMocks();
  });

  afterAll(() => {
    if (originalResizeObserver) globalThis.ResizeObserver = originalResizeObserver;
    else Reflect.deleteProperty(globalThis, "ResizeObserver");

    if (originalRequestAnimationFrame) {
      globalThis.requestAnimationFrame = originalRequestAnimationFrame;
    } else {
      Reflect.deleteProperty(globalThis, "requestAnimationFrame");
    }

    if (originalCancelAnimationFrame) {
      globalThis.cancelAnimationFrame = originalCancelAnimationFrame;
    } else {
      Reflect.deleteProperty(globalThis, "cancelAnimationFrame");
    }
  });

  it("submits a connected market order and refreshes account resources", async () => {
    renderTradeMenu();

    const submitButton = screen.getByRole("button", { name: submitLabel() });
    expect(submitButton).toBeDisabled();

    fireEvent.change(getInput("size"), { target: { value: "250" } });

    await waitFor(() => expect(submitButton).not.toBeDisabled());
    await waitFor(() =>
      expect(tradeMenuMocks.submissionParameters).toEqual(
        expect.objectContaining({
          action: "buy",
          maxSlippage: "0.0125",
          operation: "market",
          perpsPairId: "perp/btcusd",
          priceValue: "0",
          reduceOnly: false,
          sizeValue: "0.005000",
        }),
      ),
    );

    const mutateAsync = tradeMenuMocks.mutateAsync;
    fireEvent.click(submitButton);

    expect(mutateAsync).toHaveBeenCalledTimes(1);
    await waitFor(() =>
      expect(tradeMenuMocks.invalidateQueries).toHaveBeenCalledWith({
        queryKey: ["perpsTradeHistory", accountAddress],
      }),
    );
    expect(tradeMenuMocks.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsVolume", accountAddress],
    });
  });

  it("opens pair settings modals with active backend pair metadata", () => {
    tradeMenuMocks.marginModeByPair = {
      "perp/btcusd": "isolated",
    };

    renderTradeMenu();

    fireEvent.click(screen.getByRole("button", { name: "isolated" }));
    expect(tradeMenuMocks.showModal).toHaveBeenCalledWith(Modals.PerpsMarginMode, {
      pairSymbol: "BTC-USD",
      perpsPairId: "perp/btcusd",
    });

    fireEvent.click(screen.getByRole("button", { name: "50x" }));
    expect(tradeMenuMocks.showModal).toHaveBeenCalledWith(Modals.PerpsAdjustLeverage, {
      baseSymbol: "BTC",
      maxLeverage: 50,
      perpsPairId: "perp/btcusd",
    });
  });

  it("forces geoblocked traders into reduce-only submission while closable size exists", async () => {
    tradeMenuMocks.isGeoblocked = true;

    renderTradeMenu();

    expect(
      screen.getByRole("checkbox", { name: m["dex.protrade.perps.reduceOnly"]() }),
    ).toHaveAttribute("aria-checked", "true");

    const submitButton = screen.getByRole("button", { name: submitLabel() });
    fireEvent.change(getInput("size"), { target: { value: "250" } });

    await waitFor(() => expect(submitButton).not.toBeDisabled());
    await waitFor(() =>
      expect(tradeMenuMocks.submissionParameters).toEqual(
        expect.objectContaining({
          action: "buy",
          operation: "market",
          perpsPairId: "perp/btcusd",
          reduceOnly: true,
          sizeValue: "0.005000",
        }),
      ),
    );

    const mutateAsync = tradeMenuMocks.mutateAsync;
    fireEvent.click(submitButton);

    expect(mutateAsync).toHaveBeenCalledTimes(1);
  });

  it("blocks geoblocked traders from opening exposure when no reduce-only size is available", async () => {
    tradeMenuMocks.isGeoblocked = true;
    tradeMenuMocks.maxSize = 0;

    renderTradeMenu();

    const restrictedButton = screen.getByRole("button", {
      name: m["geoblock.accessRestricted"](),
    });
    expect(restrictedButton).toBeDisabled();

    fireEvent.click(restrictedButton);

    expect(tradeMenuMocks.mutateAsync).not.toHaveBeenCalled();
    await waitFor(() =>
      expect(tradeMenuMocks.submissionParameters).toEqual(
        expect.objectContaining({
          reduceOnly: true,
          sizeValue: "0.000000",
        }),
      ),
    );
  });

  it("keeps limit orders disabled until price is present and forwards time in force", async () => {
    tradeMenuMocks.orderType = "limit";

    renderTradeMenu();

    const submitButton = screen.getByRole("button", { name: submitLabel() });
    fireEvent.change(getInput("size"), { target: { value: "500" } });

    await waitFor(() => expect(submitButton).toBeDisabled());

    fireEvent.click(screen.getByRole("button", { name: "GTC" }));
    fireEvent.click(screen.getByText("Post Only"));
    fireEvent.change(getInput("price"), { target: { value: "51000" } });

    await waitFor(() => expect(submitButton).not.toBeDisabled());
    await waitFor(() =>
      expect(tradeMenuMocks.submissionParameters).toEqual(
        expect.objectContaining({
          operation: "limit",
          priceValue: "51000",
          sizeValue: "0.010000",
          timeInForce: "POST",
        }),
      ),
    );

    const mutateAsync = tradeMenuMocks.mutateAsync;
    fireEvent.click(submitButton);

    expect(mutateAsync).toHaveBeenCalledTimes(1);
  });

  it("shows TP/SL direction errors and blocks the order before submission", async () => {
    renderTradeMenu();

    const submitButton = screen.getByRole("button", { name: submitLabel() });
    fireEvent.change(getInput("size"), { target: { value: "250" } });
    await waitFor(() => expect(submitButton).not.toBeDisabled());

    fireEvent.click(screen.getByRole("checkbox", { name: m["dex.protrade.perps.tpsl"]() }));
    fireEvent.change(getInput("tpPrice"), { target: { value: "49000" } });

    expect(
      await screen.findByText(m["dex.protrade.perps.errors.tpAboveForLongs"]()),
    ).toBeInTheDocument();
    expect(submitButton).toBeDisabled();

    fireEvent.click(submitButton);

    expect(tradeMenuMocks.mutateAsync).not.toHaveBeenCalled();
  });

  it("opens authentication instead of submitting for disconnected accounts", () => {
    tradeMenuMocks.isConnected = false;

    renderTradeMenu();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["dex.protrade.spot.enableTrading"](),
      }),
    );

    expect(tradeMenuMocks.showModal).toHaveBeenCalledWith(Modals.Authenticate, {
      action: "signin",
    });
    expect(tradeMenuMocks.mutateAsync).not.toHaveBeenCalled();
  });
});
