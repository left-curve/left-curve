import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { createTestQueryClient } from "./utils/query-client";

let ProSwapEditTPSL: typeof import("../src/components/modals/ProSwapEditTPSL").ProSwapEditTPSL;

const tpslModalMocks = vi.hoisted(() => ({
  accountAddress: "0x7472616465720000000000000000000000000000",
  hideModal: vi.fn(),
  submitMutationFn: undefined as undefined | (() => Promise<unknown>),
  submitConditionalOrders: vi.fn(),
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
    selector: (state: { leverageByPair: Record<string, number> }) => unknown,
  ) =>
    selector({
      leverageByPair: {
        "perp/btcusd": 5,
      },
    }),
  useAccount: tpslModalMocks.useAccount,
  useAppConfig: () => ({
    data: {
      perpsPairs: {
        "perp/btcusd": {
          initialMarginRatio: "0.1",
        },
      },
    },
  }),
  useSigningClient: tpslModalMocks.useSigningClient,
  useStorage: () => ["0.0125"],
  useSubmitTx: ({
    mutation,
  }: {
    mutation: {
      mutationFn: () => Promise<unknown>;
      onSuccess?: () => void;
    };
  }) => {
    tpslModalMocks.submitMutationFn = mutation.mutationFn;

    return {
      isPending: false,
      mutateAsync: async () => {
        const result = await mutation.mutationFn();
        mutation.onSuccess?.();
        return result;
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

function renderEditTPSL({
  entryPrice = "100",
  markPrice = "110",
  size = "2",
}: {
  entryPrice?: string;
  markPrice?: string;
  size?: string;
} = {}) {
  return renderWithQueryClient(
    <ProSwapEditTPSL
      entryPrice={entryPrice}
      markPrice={markPrice}
      pairId="perp/btcusd"
      size={size}
      symbol="BTC"
    />,
  );
}

function getCapturedSubmitMutation() {
  if (!tpslModalMocks.submitMutationFn) {
    throw new Error("Expected TP/SL submit mutation to be captured");
  }
  return tpslModalMocks.submitMutationFn;
}

function inputByName(name: string) {
  const input = document.querySelector<HTMLInputElement>(`input[name="${name}"]`);
  if (!input) throw new Error(`Expected input named ${name}`);
  return input;
}

function dragConfiguredAmountSlider(value: number, maxValue: number) {
  const slider = screen.getByRole("slider", { name: "Slider value" });
  const track = slider.parentElement;
  if (!track) throw new Error("Expected configured amount slider track");

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

  fireEvent.mouseDown(track, { clientX: (value / maxValue) * 100 });
  fireEvent.mouseUp(window);
}

describe("ProSwapEditTPSL", () => {
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

    ({ ProSwapEditTPSL } = await import("../src/components/modals/ProSwapEditTPSL"));
    consoleError.mockRestore();
  }, 30000);

  afterAll(() => {
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: tpslModalMocks.hideModal,
    });
    tpslModalMocks.submitMutationFn = undefined;
    tpslModalMocks.submitConditionalOrders.mockResolvedValue(undefined);
    tpslModalMocks.useAccount.mockReturnValue({
      account: {
        address: tpslModalMocks.accountAddress,
      },
    });
    tpslModalMocks.useSigningClient.mockReturnValue({
      data: {
        submitConditionalOrders: tpslModalMocks.submitConditionalOrders,
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("submits long-position take-profit and stop-loss conditional orders", async () => {
    const { invalidateQueries } = renderEditTPSL();

    fireEvent.change(inputByName("tpPrice"), {
      target: { value: "130" },
    });
    fireEvent.change(inputByName("slPrice"), {
      target: { value: "95" },
    });

    expect(inputByName("tpPercent")).toHaveValue("150");
    expect(inputByName("slPercent")).toHaveValue("25");

    fireEvent.click(screen.getByRole("button", { name: m["modals.tpsl.confirm"]() }));

    await waitFor(() => {
      expect(tpslModalMocks.submitConditionalOrders).toHaveBeenCalledWith({
        orders: [
          {
            maxSlippage: "0.0125",
            pairId: "perp/btcusd",
            triggerDirection: "above",
            triggerPrice: "130",
          },
          {
            maxSlippage: "0.0125",
            pairId: "perp/btcusd",
            triggerDirection: "below",
            triggerPrice: "95",
          },
        ],
        sender: "0x7472616465720000000000000000000000000000",
      });
    });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["prices"] });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: ["perpsTradeHistory", "0x7472616465720000000000000000000000000000"],
    });
    expect(tpslModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("does not close or invalidate resources when backend TP/SL submission fails", async () => {
    const { invalidateQueries } = renderEditTPSL();
    tpslModalMocks.submitConditionalOrders.mockRejectedValueOnce(
      new Error("conditional order rejected"),
    );

    fireEvent.change(inputByName("tpPrice"), {
      target: { value: "130" },
    });
    fireEvent.change(inputByName("slPrice"), {
      target: { value: "95" },
    });

    await expect(getCapturedSubmitMutation()()).rejects.toThrow("conditional order rejected");

    expect(tpslModalMocks.submitConditionalOrders).toHaveBeenCalledWith({
      orders: [
        {
          maxSlippage: "0.0125",
          pairId: "perp/btcusd",
          triggerDirection: "above",
          triggerPrice: "130",
        },
        {
          maxSlippage: "0.0125",
          pairId: "perp/btcusd",
          triggerDirection: "below",
          triggerPrice: "95",
        },
      ],
      sender: "0x7472616465720000000000000000000000000000",
    });
    expect(invalidateQueries).not.toHaveBeenCalled();
    expect(tpslModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("does not build TP/SL orders without a signing client", async () => {
    tpslModalMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    renderEditTPSL();

    fireEvent.change(inputByName("tpPrice"), {
      target: { value: "130" },
    });

    await expect(getCapturedSubmitMutation()()).rejects.toThrow("No signing client available");
    expect(tpslModalMocks.submitConditionalOrders).not.toHaveBeenCalled();
    expect(tpslModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("does not build TP/SL orders without a connected account", async () => {
    tpslModalMocks.useAccount.mockReturnValue({
      account: null,
    });

    renderEditTPSL();

    fireEvent.change(inputByName("tpPrice"), {
      target: { value: "130" },
    });

    await expect(getCapturedSubmitMutation()()).rejects.toThrow("No account found");
    expect(tpslModalMocks.submitConditionalOrders).not.toHaveBeenCalled();
    expect(tpslModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("blocks a long take-profit trigger that would fire at the current mark price", () => {
    renderEditTPSL();

    fireEvent.change(inputByName("tpPrice"), {
      target: { value: "105" },
    });

    expect(screen.getByText(m["modals.tpsl.errors.tpAboveForLongs"]())).toBeInTheDocument();
    expect(screen.getByRole("button", { name: m["modals.tpsl.confirm"]() })).toBeDisabled();
    expect(tpslModalMocks.submitConditionalOrders).not.toHaveBeenCalled();
  });

  it("submits a configured partial-size short take-profit order", async () => {
    renderEditTPSL({
      markPrice: "95",
      size: "-4",
    });

    fireEvent.change(inputByName("tpPrice"), {
      target: { value: "80" },
    });
    fireEvent.click(screen.getByRole("checkbox", { name: m["modals.tpsl.configureAmount"]() }));
    dragConfiguredAmountSlider(1.5, 4);
    fireEvent.click(screen.getByRole("button", { name: m["modals.tpsl.confirm"]() }));

    await waitFor(() => {
      expect(tpslModalMocks.submitConditionalOrders).toHaveBeenCalledWith({
        orders: [
          {
            maxSlippage: "0.0125",
            pairId: "perp/btcusd",
            size: "1.500000",
            triggerDirection: "below",
            triggerPrice: "80",
          },
        ],
        sender: "0x7472616465720000000000000000000000000000",
      });
    });
  });

  it("does not submit conditional orders when configured size is zero", () => {
    renderEditTPSL();

    fireEvent.change(inputByName("tpPrice"), {
      target: { value: "130" },
    });
    fireEvent.click(screen.getByRole("checkbox", { name: m["modals.tpsl.configureAmount"]() }));
    dragConfiguredAmountSlider(0, 2);

    const confirmButton = screen.getByRole("button", { name: m["modals.tpsl.confirm"]() });
    expect(confirmButton).toBeDisabled();

    fireEvent.click(confirmButton);

    expect(tpslModalMocks.submitConditionalOrders).not.toHaveBeenCalled();
    expect(tpslModalMocks.hideModal).not.toHaveBeenCalled();
  });
});
