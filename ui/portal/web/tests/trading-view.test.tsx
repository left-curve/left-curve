import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetAppletsKitMocks, setAppletsKitUseTheme } from "./mocks/applets-kit";

import { TradingView } from "../src/components/dex/components/TradingView";
import { createQueryClientWrapper } from "./utils/query-client";

const tradingViewMocks = vi.hoisted(() => ({
  createPerpsDataFeed: vi.fn(),
  datafeed: { name: "perps-datafeed" },
  emitterHandlers: new Map<string, (...args: unknown[]) => void>(),
  emitterOff: vi.fn(),
  emitterOn: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
    tradingViewMocks.emitterHandlers.set(event, handler);
  }),
  orders: {
    "bid-btc": {
      limitPrice: "30000",
      pairId: "perp/btcusd",
      size: "1",
    },
    "short-eth": {
      limitPrice: "2225",
      pairId: "perp/ethusd",
      size: "-0.4",
    },
  },
  position: {
    "perp/btcusd": {
      entryPrice: "29250",
      liquidationPrice: "35500",
      size: "-0.25",
    },
    "perp/ethusd": {
      conditionalOrderAbove: {
        triggerPrice: "2500",
      },
      conditionalOrderBelow: {
        triggerPrice: "1800",
      },
      entryPrice: "2100",
      liquidationPrice: "1600",
      size: "0.75",
    },
  },
  publicClient: {
    subscribe: {
      emitter: {
        off: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
          tradingViewMocks.emitterOff(event, handler);
        }),
        on: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
          tradingViewMocks.emitterOn(event, handler);
        }),
      },
    },
  },
  subscriptions: {
    subscribe: vi.fn(),
  },
  usePerpsOrdersByUser: vi.fn(),
  usePerpsUserStateExtended: vi.fn(),
  widgetInstances: [] as Array<{
    applyOverrides: ReturnType<typeof vi.fn>;
    chartApi: {
      clearMarks: ReturnType<typeof vi.fn>;
      createShape: ReturnType<typeof vi.fn>;
      createStudy: ReturnType<typeof vi.fn>;
      getAllShapes: ReturnType<typeof vi.fn>;
      getAllStudies: ReturnType<typeof vi.fn>;
      getShapeById: ReturnType<typeof vi.fn>;
      refreshMarks: ReturnType<typeof vi.fn>;
      removeEntity: ReturnType<typeof vi.fn>;
      resetData: ReturnType<typeof vi.fn>;
      setSymbol: ReturnType<typeof vi.fn>;
      symbol: ReturnType<typeof vi.fn>;
    };
    options: Record<string, unknown>;
    remove: ReturnType<typeof vi.fn>;
    resetCache: ReturnType<typeof vi.fn>;
    save: ReturnType<typeof vi.fn>;
    subscribe: ReturnType<typeof vi.fn>;
  }>,
  widgetMock: vi.fn(),
}));

vi.mock("~/datafeed", () => ({
  createPerpsDataFeed: tradingViewMocks.createPerpsDataFeed,
}));

vi.mock("@left-curve/tradingview", () => ({
  widget: tradingViewMocks.widgetMock,
}));

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();
  const appState = {
    settings: {
      timeFormat: "hh:mm a",
      timeZone: "utc",
    },
  };

  return {
    ...actual,
    useApp: (selector?: (state: typeof appState) => unknown) =>
      typeof selector === "function" ? selector(appState) : appState,
  };
});

vi.mock("@left-curve/store", () => ({
  useConfig: () => ({
    subscriptions: tradingViewMocks.subscriptions,
  }),
  usePerpsOrdersByUser: tradingViewMocks.usePerpsOrdersByUser,
  usePerpsUserStateExtended: tradingViewMocks.usePerpsUserStateExtended,
  usePublicClient: () => tradingViewMocks.publicClient,
}));

describe("TradingView", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseTheme({
      theme: "dark",
    });
    localStorage.clear();
    tradingViewMocks.emitterHandlers.clear();
    tradingViewMocks.widgetInstances = [];
    tradingViewMocks.createPerpsDataFeed.mockReturnValue(tradingViewMocks.datafeed);
    tradingViewMocks.usePerpsUserStateExtended.mockImplementation((selector) =>
      selector({
        positions: tradingViewMocks.position,
      }),
    );
    tradingViewMocks.usePerpsOrdersByUser.mockImplementation((selector) =>
      selector({
        orders: tradingViewMocks.orders,
      }),
    );
    tradingViewMocks.widgetMock.mockImplementation(function widget(
      options: Record<string, unknown>,
    ) {
      let activeSymbol = options.symbol as string;
      const chartApi = {
        clearMarks: vi.fn(),
        createShape: vi.fn().mockResolvedValue(undefined),
        createStudy: vi.fn(),
        getAllShapes: vi.fn(() => []),
        getAllStudies: vi.fn(() => []),
        getShapeById: vi.fn(() => ({
          isSavingEnabled: () => true,
        })),
        refreshMarks: vi.fn(),
        removeEntity: vi.fn(),
        resetData: vi.fn(),
        setSymbol: vi.fn((symbol: string, callback?: () => void) => {
          activeSymbol = symbol;
          callback?.();
        }),
        symbol: vi.fn(() => activeSymbol),
      };
      const instance = {
        applyOverrides: vi.fn(),
        chart: vi.fn(() => chartApi),
        chartApi,
        onChartReady: vi.fn((callback: () => void) => callback()),
        options,
        remove: vi.fn(),
        resetCache: vi.fn(),
        save: vi.fn((callback: (state: unknown) => void) =>
          callback({ panes: [{ sources: ["saved-layout"] }] }),
        ),
        subscribe: vi.fn(),
      };

      tradingViewMocks.widgetInstances.push(instance);

      return instance;
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    localStorage.clear();
  });

  it("wires the backend datafeed, saved layout, reconnect invalidation, autosave, and overlay lines", async () => {
    const savedLayout = { panes: [{ sources: ["restored-layout"] }] };
    localStorage.setItem("tv_v4.ETH-USD_perps", JSON.stringify(savedLayout));
    const Wrapper = createQueryClientWrapper();
    const { unmount } = render(
      <TradingView
        accountAddress="0x1234567890abcdef1234567890abcdef12345678"
        coins={{
          base: { symbol: "ETH" },
          quote: { symbol: "USD" },
        }}
        perpsPairId="perp/ethusd"
      />,
      { wrapper: Wrapper },
    );

    const widget = tradingViewMocks.widgetInstances[0];
    expect(widget.options).toMatchObject({
      datafeed: tradingViewMocks.datafeed,
      saved_data: savedLayout,
      symbol: "ETH-USD",
      theme: "dark",
    });
    expect(localStorage.getItem("tradingview.time_hours_format")).toBe("12-hours");
    expect(tradingViewMocks.createPerpsDataFeed).toHaveBeenCalledWith(
      expect.objectContaining({
        client: tradingViewMocks.publicClient,
        subscriptions: tradingViewMocks.subscriptions,
      }),
    );
    expect(tradingViewMocks.createPerpsDataFeed.mock.calls[0]?.[0].queryClient).toBeTruthy();
    expect(tradingViewMocks.usePerpsUserStateExtended).toHaveBeenCalledWith(expect.any(Function), {
      accountAddress: "0x1234567890abcdef1234567890abcdef12345678",
    });
    expect(tradingViewMocks.usePerpsOrdersByUser).toHaveBeenCalledWith(
      expect.any(Function),
      {
        accountAddress: "0x1234567890abcdef1234567890abcdef12345678",
      },
      expect.any(Function),
    );
    expect(widget.chartApi.createStudy).toHaveBeenCalledWith("Volume", false, false);
    expect(widget.applyOverrides).toHaveBeenCalledWith(
      expect.objectContaining({
        timezone: "Etc/UTC",
      }),
    );

    tradingViewMocks.emitterHandlers.get("connected")?.();
    expect(widget.resetCache).toHaveBeenCalledOnce();
    expect(widget.chartApi.resetData).toHaveBeenCalledOnce();

    const autosave = widget.subscribe.mock.calls.find(
      ([event]) => event === "onAutoSaveNeeded",
    )?.[1];
    expect(autosave).toEqual(expect.any(Function));
    autosave();
    expect(localStorage.getItem("tv_v4.ETH-USD_perps")).toBe(
      JSON.stringify({ panes: [{ sources: ["saved-layout"] }] }),
    );

    await waitFor(() => {
      expect(widget.chartApi.createShape).toHaveBeenCalledWith(
        {
          price: 2100,
          time: expect.any(Number),
        },
        expect.objectContaining({
          overrides: expect.objectContaining({
            linecolor: "#27AE60",
            linestyle: 0,
          }),
        }),
      );
      expect(widget.chartApi.createShape).toHaveBeenCalledWith(
        {
          price: 2225,
          time: expect.any(Number),
        },
        expect.objectContaining({
          overrides: expect.objectContaining({
            linecolor: "#EB5757",
            linestyle: 2,
          }),
        }),
      );
    });
    expect(widget.chartApi.createShape).not.toHaveBeenCalledWith(
      expect.objectContaining({ price: 30000 }),
      expect.anything(),
    );

    unmount();

    expect(tradingViewMocks.emitterOff).toHaveBeenCalledWith(
      "connected",
      tradingViewMocks.emitterHandlers.get("connected"),
    );
    expect(widget.remove).toHaveBeenCalledOnce();
  });

  it("switches the active symbol and redraws overlays from the new backend pair without recreating the widget", async () => {
    const Wrapper = createQueryClientWrapper();
    const { rerender } = render(
      <TradingView
        accountAddress="0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        coins={{
          base: { symbol: "ETH" },
          quote: { symbol: "USD" },
        }}
        perpsPairId="perp/ethusd"
      />,
      { wrapper: Wrapper },
    );
    const widget = tradingViewMocks.widgetInstances[0];

    await waitFor(() => {
      expect(widget.chartApi.createShape).toHaveBeenCalledWith(
        expect.objectContaining({ price: 2100 }),
        expect.anything(),
      );
    });
    widget.chartApi.createShape.mockClear();

    rerender(
      <TradingView
        accountAddress="0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        coins={{
          base: { symbol: "BTC" },
          quote: { symbol: "USD" },
        }}
        perpsPairId="perp/btcusd"
      />,
    );

    expect(tradingViewMocks.widgetMock).toHaveBeenCalledOnce();
    expect(widget.chartApi.setSymbol).toHaveBeenCalledWith("BTC-USD", expect.any(Function));

    await waitFor(() => {
      expect(widget.chartApi.createShape).toHaveBeenCalledWith(
        expect.objectContaining({ price: 29250 }),
        expect.objectContaining({
          overrides: expect.objectContaining({
            linecolor: "#EB5757",
            linestyle: 0,
          }),
        }),
      );
      expect(widget.chartApi.createShape).toHaveBeenCalledWith(
        expect.objectContaining({ price: 30000 }),
        expect.objectContaining({
          overrides: expect.objectContaining({
            linecolor: "#27AE60",
            linestyle: 2,
          }),
        }),
      );
    });
    expect(widget.chartApi.createShape).not.toHaveBeenCalledWith(
      expect.objectContaining({ price: 2225 }),
      expect.anything(),
    );
  });
});
