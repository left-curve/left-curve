import { QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { createTestQueryClient } from "./utils/query-client";

let PerpsClosePosition: typeof import("../src/components/modals/PerpsClosePosition").PerpsClosePosition;
let PerpsCloseAll: typeof import("../src/components/modals/PerpsCloseAll").PerpsCloseAll;
let PerpsCloseOrder: typeof import("../src/components/modals/PerpsCloseOrder").PerpsCloseOrder;
let PerpsAdjustLeverage: typeof import("../src/components/modals/PerpsAdjustLeverage").PerpsAdjustLeverage;
let PerpsMarginMode: typeof import("../src/components/modals/PerpsMarginMode").PerpsMarginMode;
let ProSwapEditedSL: typeof import("../src/components/modals/ProSwapEditedSL").ProSwapEditedSL;

const perpsModalMocks = vi.hoisted(() => ({
  accountAddress: "0x7472616465720000000000000000000000000000",
  cancelPerpsOrder: vi.fn(),
  cancelConditionalOrder: vi.fn(),
  hideModal: vi.fn(),
  leverageByPair: {} as Record<string, number>,
  marginModeByPair: {} as Record<string, "cross" | "isolated">,
  setLeverage: vi.fn(),
  setMarginMode: vi.fn(),
  showModal: vi.fn(),
  submitMutationFn: undefined as undefined | ((variables?: unknown) => Promise<unknown>),
  submitPerpsOrder: vi.fn(),
  useAccount: vi.fn(),
  useSigningClient: vi.fn(),
}));

vi.mock("../src/components/modals/TPSLPositionInfo", () => ({
  TPSLPositionInfo: ({
    absSize,
    entryPrice,
    isLong,
    markPrice,
    symbol,
  }: {
    absSize: number;
    entryPrice: string;
    isLong: boolean;
    markPrice: string;
    symbol: string;
  }) => (
    <output data-testid="tpsl-position-info">
      {symbol}:{isLong ? "long" : "short"}:{absSize}:{entryPrice}:{markPrice}
    </output>
  ),
}));

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
  perpsTradeSettingsStore: (
    selector: (state: {
      leverageByPair: Record<string, number>;
      marginModeByPair: Record<string, "cross" | "isolated">;
      setLeverage: typeof perpsModalMocks.setLeverage;
      setMarginMode: typeof perpsModalMocks.setMarginMode;
    }) => unknown,
  ) =>
    selector({
      leverageByPair: perpsModalMocks.leverageByPair,
      marginModeByPair: perpsModalMocks.marginModeByPair,
      setLeverage: perpsModalMocks.setLeverage,
      setMarginMode: perpsModalMocks.setMarginMode,
    }),
  useAccount: perpsModalMocks.useAccount,
  useConfig: () => ({
    coins: {
      byDenom: {
        "bridge/btc": {
          decimals: 8,
          denom: "bridge/btc",
          logoURI: "/btc.png",
          symbol: "BTC",
        },
      },
    },
  }),
  useSigningClient: perpsModalMocks.useSigningClient,
  useStorage: () => ["0.0125"],
  useSubmitTx: ({
    mutation,
  }: {
    mutation: {
      mutationFn: (variables?: unknown) => Promise<unknown>;
      onError?: () => void;
      onSuccess?: () => void;
    };
  }) => {
    perpsModalMocks.submitMutationFn = mutation.mutationFn;

    return {
      isPending: false,
      mutateAsync: async (variables?: unknown) => {
        try {
          const result = await mutation.mutationFn(variables);
          mutation.onSuccess?.();
          return result;
        } catch (error) {
          mutation.onError?.();
          throw error;
        }
      },
    };
  },
}));

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();
  const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

  render(<QueryClientProvider client={queryClient}>{component}</QueryClientProvider>);

  return {
    invalidateQueries,
  };
}

function getCapturedSubmitMutation() {
  if (!perpsModalMocks.submitMutationFn) {
    throw new Error("Expected perps modal submit mutation to be captured");
  }
  return perpsModalMocks.submitMutationFn;
}

function closeSizeInput() {
  const input = screen.getAllByRole("textbox")[0];
  if (!input) throw new Error("Expected close size input");
  return input;
}

function slider() {
  return screen.getByRole("slider", { name: "Slider value" });
}

function dragSlider(value: number, { max, min }: { max: number; min: number }) {
  const thumb = slider();
  const track = thumb.parentElement;
  if (!track) throw new Error("Expected slider track");

  Object.defineProperty(track, "getBoundingClientRect", {
    configurable: true,
    value: () => ({
      bottom: 4,
      height: 4,
      left: 0,
      right: 100,
      top: 0,
      width: 100,
      x: 0,
      y: 0,
      toJSON: () => undefined,
    }),
  });

  fireEvent.mouseDown(track, { clientX: ((value - min) / (max - min)) * 100 });
  fireEvent.mouseUp(window);
}

describe("perps modals", () => {
  beforeAll(async () => {
    const originalConsoleError = console.error;
    const consoleError = vi.spyOn(console, "error").mockImplementation((message, ...args) => {
      if (
        typeof message === "string" &&
        message.includes("forwardRef render functions accept exactly two parameters")
      ) {
        return;
      }
      originalConsoleError(message, ...args);
    });

    ({ PerpsAdjustLeverage } = await import("../src/components/modals/PerpsAdjustLeverage"));
    ({ PerpsCloseAll } = await import("../src/components/modals/PerpsCloseAll"));
    ({ PerpsCloseOrder } = await import("../src/components/modals/PerpsCloseOrder"));
    ({ PerpsClosePosition } = await import("../src/components/modals/PerpsClosePosition"));
    ({ PerpsMarginMode } = await import("../src/components/modals/PerpsMarginMode"));
    ({ ProSwapEditedSL } = await import("../src/components/modals/ProSwapEditedSL"));
    consoleError.mockRestore();
  }, 30000);

  afterAll(() => {
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: perpsModalMocks.hideModal,
      settings: {
        formatNumberOptions: {
          currency: "USD",
          language: "en-US",
          mask: 1,
        },
      },
      showModal: perpsModalMocks.showModal,
    });
    perpsModalMocks.submitMutationFn = undefined;
    perpsModalMocks.cancelConditionalOrder.mockResolvedValue(undefined);
    perpsModalMocks.cancelPerpsOrder.mockResolvedValue(undefined);
    perpsModalMocks.leverageByPair = {};
    perpsModalMocks.marginModeByPair = {};
    perpsModalMocks.submitPerpsOrder.mockResolvedValue(undefined);
    perpsModalMocks.useAccount.mockReturnValue({
      account: {
        address: perpsModalMocks.accountAddress,
      },
    });
    perpsModalMocks.useSigningClient.mockReturnValue({
      data: {
        cancelConditionalOrder: perpsModalMocks.cancelConditionalOrder,
        cancelPerpsOrder: perpsModalMocks.cancelPerpsOrder,
        submitPerpsOrder: perpsModalMocks.submitPerpsOrder,
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("submits a reduce-only market order for a partial long-position close", async () => {
    const { invalidateQueries } = renderWithQueryClient(
      <PerpsClosePosition pairId="perp/btcusd" pnl={0} size="4" />,
    );

    expect(closeSizeInput()).toHaveValue("4");

    dragSlider(50, { min: 0, max: 100 });
    fireEvent.click(screen.getByRole("button", { name: m["modals.marketClose.confirm"]() }));

    await waitFor(() => {
      expect(perpsModalMocks.submitPerpsOrder).toHaveBeenCalledWith({
        kind: {
          market: {
            maxSlippage: "0.0125",
          },
        },
        pairId: "perp/btcusd",
        reduceOnly: true,
        sender: "0x7472616465720000000000000000000000000000",
        size: "-2",
      });
    });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["prices"] });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x7472616465720000000000000000000000000000"],
    });
    expect(perpsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("does not close or invalidate resources when backend close-position submission fails", async () => {
    const { invalidateQueries } = renderWithQueryClient(
      <PerpsClosePosition pairId="perp/btcusd" pnl={0} size="4" />,
    );
    perpsModalMocks.submitPerpsOrder.mockRejectedValueOnce(new Error("market close rejected"));

    dragSlider(50, { min: 0, max: 100 });

    await expect(getCapturedSubmitMutation()()).rejects.toThrow("market close rejected");

    expect(perpsModalMocks.submitPerpsOrder).toHaveBeenCalledWith({
      kind: {
        market: {
          maxSlippage: "0.0125",
        },
      },
      pairId: "perp/btcusd",
      reduceOnly: true,
      sender: "0x7472616465720000000000000000000000000000",
      size: "-2",
    });
    expect(invalidateQueries).not.toHaveBeenCalled();
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("does not submit a close-position order for a zero close size", () => {
    renderWithQueryClient(<PerpsClosePosition pairId="perp/btcusd" pnl={0} size="4" />);

    fireEvent.change(closeSizeInput(), {
      target: { value: "0" },
    });

    const confirmButton = screen.getByRole("button", { name: m["modals.marketClose.confirm"]() });
    expect(confirmButton).toBeDisabled();

    fireEvent.click(confirmButton);

    expect(perpsModalMocks.submitPerpsOrder).not.toHaveBeenCalled();
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("cancels one open perps order with the backend order id request", async () => {
    const { invalidateQueries } = renderWithQueryClient(<PerpsCloseOrder orderId="order-123" />);

    fireEvent.click(screen.getByRole("button", { name: m["modals.proTradeCloseOrder.action"]() }));

    await waitFor(() => {
      expect(perpsModalMocks.cancelPerpsOrder).toHaveBeenCalledWith({
        request: {
          one: "order-123",
        },
        sender: "0x7472616465720000000000000000000000000000",
      });
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x7472616465720000000000000000000000000000"],
    });
    expect(perpsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("does not close or invalidate resources when backend single-order cancellation fails", async () => {
    const { invalidateQueries } = renderWithQueryClient(<PerpsCloseOrder orderId="order-123" />);
    perpsModalMocks.cancelPerpsOrder.mockRejectedValueOnce(new Error("single cancel rejected"));

    await expect(getCapturedSubmitMutation()()).rejects.toThrow("single cancel rejected");

    expect(perpsModalMocks.cancelPerpsOrder).toHaveBeenCalledWith({
      request: {
        one: "order-123",
      },
      sender: "0x7472616465720000000000000000000000000000",
    });
    expect(invalidateQueries).not.toHaveBeenCalled();
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("cancels every open perps order with the backend all-orders request", async () => {
    const { invalidateQueries } = renderWithQueryClient(<PerpsCloseAll />);

    fireEvent.click(
      screen.getByRole("button", { name: m["modals.protradeCloseAllOrders.action"]() }),
    );

    await waitFor(() => {
      expect(perpsModalMocks.cancelPerpsOrder).toHaveBeenCalledWith({
        request: "all",
        sender: "0x7472616465720000000000000000000000000000",
      });
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x7472616465720000000000000000000000000000"],
    });
    expect(perpsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("does not close or invalidate resources when backend all-orders cancellation fails", async () => {
    const { invalidateQueries } = renderWithQueryClient(<PerpsCloseAll />);
    perpsModalMocks.cancelPerpsOrder.mockRejectedValueOnce(new Error("all cancel rejected"));

    await expect(getCapturedSubmitMutation()()).rejects.toThrow("all cancel rejected");

    expect(perpsModalMocks.cancelPerpsOrder).toHaveBeenCalledWith({
      request: "all",
      sender: "0x7472616465720000000000000000000000000000",
    });
    expect(invalidateQueries).not.toHaveBeenCalled();
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("clamps stored leverage to the pair max and persists the rounded slider value", () => {
    perpsModalMocks.leverageByPair = {
      "perp/btcusd": 200,
    };

    renderWithQueryClient(
      <PerpsAdjustLeverage baseSymbol="BTC" maxLeverage={50} perpsPairId="perp/btcusd" />,
    );

    expect(screen.getByText(m["modals.adjustLeverage.title"]())).toBeInTheDocument();
    expect(
      screen.getByText(
        m["modals.adjustLeverage.description"]({
          maxLeverage: "50",
          symbol: "BTC",
        }),
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(m["modals.adjustLeverage.warning"]())).toBeInTheDocument();
    expect(slider()).toHaveAttribute("aria-valuenow", "50");

    dragSlider(12.6, { min: 1, max: 50 });
    fireEvent.click(screen.getByRole("button", { name: m["modals.adjustLeverage.confirm"]() }));

    expect(perpsModalMocks.setLeverage).toHaveBeenCalledWith("perp/btcusd", 13, 50);
    expect(perpsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("defaults missing leverage to max leverage and clamps a too-low stored value to one", () => {
    renderWithQueryClient(
      <PerpsAdjustLeverage baseSymbol="ETH" maxLeverage={25} perpsPairId="perp/ethusd" />,
    );

    expect(slider()).toHaveAttribute("aria-valuenow", "25");

    cleanup();
    vi.clearAllMocks();
    perpsModalMocks.leverageByPair = {
      "perp/ethusd": -4,
    };

    renderWithQueryClient(
      <PerpsAdjustLeverage baseSymbol="ETH" maxLeverage={25} perpsPairId="perp/ethusd" />,
    );

    expect(slider()).toHaveAttribute("aria-valuenow", "1");
  });

  it("persists cross margin mode and keeps isolated margin disabled", () => {
    perpsModalMocks.marginModeByPair = {
      "perp/btcusd": "isolated",
    };

    renderWithQueryClient(<PerpsMarginMode pairSymbol="BTC-USD" perpsPairId="perp/btcusd" />);

    expect(
      screen.getByText(m["modals.marginMode.title"]({ symbol: "BTC-USD" })),
    ).toBeInTheDocument();
    expect(screen.getByText(m["modals.marginMode.crossDescription"]())).toBeInTheDocument();
    expect(screen.getByText(m["modals.marginMode.isolatedDescription"]())).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: (name) => name.includes(m["modals.marginMode.isolated"]()),
      }),
    ).toBeDisabled();

    fireEvent.click(
      screen.getByRole("button", {
        name: (name) => name.includes(m["modals.marginMode.cross"]()),
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: m["modals.marginMode.confirm"]() }));

    expect(perpsModalMocks.setMarginMode).toHaveBeenCalledWith("perp/btcusd", "cross");
    expect(perpsModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("clamps short-position close size to the open position amount", async () => {
    renderWithQueryClient(<PerpsClosePosition pairId="perp/btcusd" pnl={0} size="-3" />);

    fireEvent.change(closeSizeInput(), {
      target: { value: "9" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["modals.marketClose.confirm"]() }));

    await waitFor(() => {
      expect(perpsModalMocks.submitPerpsOrder).toHaveBeenCalledWith(
        expect.objectContaining({
          pairId: "perp/btcusd",
          reduceOnly: true,
          size: "3",
        }),
      );
    });
  });

  it("cancels an existing long-position take-profit order by trigger direction", async () => {
    const { invalidateQueries } = renderWithQueryClient(
      <ProSwapEditedSL
        conditionalOrderAbove={{ triggerPrice: "150" } as never}
        conditionalOrderBelow={{ triggerPrice: "90" } as never}
        entryPrice="100"
        markPrice="110"
        pairId="perp/btcusd"
        size="2"
        symbol="BTC"
      />,
    );

    fireEvent.click(screen.getAllByRole("button", { name: m["modals.tpsl.cancel"]() })[0]);

    await waitFor(() => {
      expect(perpsModalMocks.cancelConditionalOrder).toHaveBeenCalledWith({
        request: {
          one: {
            pairId: "perp/btcusd",
            triggerDirection: "above",
          },
        },
        sender: "0x7472616465720000000000000000000000000000",
      });
    });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["prices"] });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x7472616465720000000000000000000000000000"],
    });
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("cancels an existing short-position take-profit order with the flipped trigger direction", async () => {
    const { invalidateQueries } = renderWithQueryClient(
      <ProSwapEditedSL
        conditionalOrderAbove={{ triggerPrice: "120" } as never}
        conditionalOrderBelow={{ triggerPrice: "80" } as never}
        entryPrice="100"
        markPrice="90"
        pairId="perp/btcusd"
        size="-2"
        symbol="BTC"
      />,
    );

    expect(screen.getByTestId("tpsl-position-info")).toHaveTextContent("BTC:short:2:100:90");

    fireEvent.click(screen.getAllByRole("button", { name: m["modals.tpsl.cancel"]() })[0]);

    await waitFor(() => {
      expect(perpsModalMocks.cancelConditionalOrder).toHaveBeenCalledWith({
        request: {
          one: {
            pairId: "perp/btcusd",
            triggerDirection: "below",
          },
        },
        sender: "0x7472616465720000000000000000000000000000",
      });
    });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["prices"] });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x7472616465720000000000000000000000000000"],
    });
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("does not invalidate resources when backend TP/SL cancellation fails", async () => {
    const { invalidateQueries } = renderWithQueryClient(
      <ProSwapEditedSL
        conditionalOrderAbove={{ triggerPrice: "150" } as never}
        conditionalOrderBelow={{ triggerPrice: "90" } as never}
        entryPrice="100"
        markPrice="110"
        pairId="perp/btcusd"
        size="2"
        symbol="BTC"
      />,
    );
    perpsModalMocks.cancelConditionalOrder.mockRejectedValueOnce(
      new Error("conditional cancel rejected"),
    );

    let caughtError: unknown;
    await act(async () => {
      try {
        await getCapturedSubmitMutation()("above");
      } catch (error) {
        caughtError = error;
      }
    });

    expect(caughtError).toBeInstanceOf(Error);
    expect((caughtError as Error).message).toBe("conditional cancel rejected");
    expect(perpsModalMocks.cancelConditionalOrder).toHaveBeenCalledWith({
      request: {
        one: {
          pairId: "perp/btcusd",
          triggerDirection: "above",
        },
      },
      sender: "0x7472616465720000000000000000000000000000",
    });
    expect(invalidateQueries).not.toHaveBeenCalled();
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("does not cancel TP/SL conditional orders without a signing client", async () => {
    perpsModalMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    renderWithQueryClient(
      <ProSwapEditedSL
        conditionalOrderAbove={{ triggerPrice: "150" } as never}
        conditionalOrderBelow={{ triggerPrice: "90" } as never}
        entryPrice="100"
        markPrice="110"
        pairId="perp/btcusd"
        size="2"
        symbol="BTC"
      />,
    );

    await expect(getCapturedSubmitMutation()("above")).rejects.toThrow(
      "No signing client available",
    );
    expect(perpsModalMocks.cancelConditionalOrder).not.toHaveBeenCalled();
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("does not cancel TP/SL conditional orders without a connected account", async () => {
    perpsModalMocks.useAccount.mockReturnValue({
      account: null,
    });

    renderWithQueryClient(
      <ProSwapEditedSL
        conditionalOrderAbove={{ triggerPrice: "150" } as never}
        conditionalOrderBelow={{ triggerPrice: "90" } as never}
        entryPrice="100"
        markPrice="110"
        pairId="perp/btcusd"
        size="2"
        symbol="BTC"
      />,
    );

    await expect(getCapturedSubmitMutation()("above")).rejects.toThrow("No account found");
    expect(perpsModalMocks.cancelConditionalOrder).not.toHaveBeenCalled();
    expect(perpsModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("opens the edit TP/SL modal with the current position and conditional orders", () => {
    const conditionalOrderAbove = { triggerPrice: "120" } as never;
    const conditionalOrderBelow = { triggerPrice: "80" } as never;

    renderWithQueryClient(
      <ProSwapEditedSL
        conditionalOrderAbove={conditionalOrderAbove}
        conditionalOrderBelow={conditionalOrderBelow}
        entryPrice="100"
        markPrice="98"
        pairId="perp/btcusd"
        size="-2"
        symbol="BTC"
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: m["modals.tpsl.edit"]() }));

    expect(perpsModalMocks.showModal).toHaveBeenCalledWith("pro-edit-tpsl", {
      conditionalOrderAbove,
      conditionalOrderBelow,
      entryPrice: "100",
      markPrice: "98",
      pairId: "perp/btcusd",
      size: "-2",
      symbol: "BTC",
    });
  });
});
