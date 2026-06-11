import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";

import { BridgeWithdraw } from "../src/components/modals/BridgeWithdraw";

const bridgeModalMocks = vi.hoisted(() => ({
  getPrice: vi.fn(),
  hideModal: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  usePrices: () => ({
    getPrice: bridgeModalMocks.getPrice,
  }),
}));

const usdcCoin = {
  decimals: 6,
  denom: "bridge/usdc",
  logoURI: "/usdc.png",
  name: "USD Coin",
  symbol: "USDC",
};

const bridgeConfig = {
  chain: {
    blockExplorers: {
      default: {
        url: "https://explorer.example",
      },
    },
    id: "11155111",
    name: "Sepolia",
  },
};

function createMutation() {
  return {
    isPending: false,
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
  };
}

function getIconOnlyButton(container: HTMLElement) {
  const button = Array.from(container.querySelectorAll<HTMLButtonElement>("button")).find(
    (candidate) => candidate.textContent?.trim() === "",
  );
  if (!button) throw new Error("Expected an icon-only modal button to exist");
  return button;
}

describe("bridge confirmation modals", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: bridgeModalMocks.hideModal,
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
    });
    bridgeModalMocks.getPrice.mockImplementation((amount: string, denom: string) => {
      return `$${amount}:${denom}`;
    });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.clearAllMocks();
  });

  it("submits a bridge withdrawal and closes the modal from the confirmation action", () => {
    const withdraw = createMutation();
    const recipient = "0x4444444444444444444444444444444444444444";

    const { container } = render(
      <BridgeWithdraw
        amount="3.25"
        coin={usdcCoin}
        config={bridgeConfig}
        fee="0.25"
        recipient={recipient}
        withdraw={withdraw}
      />,
    );

    expect(screen.getByRole("heading", { name: m["bridge.withdraw.title"]() })).toBeInTheDocument();
    expect(screen.getByText("3.25 USDC")).toBeInTheDocument();
    expect(screen.getByText("$3.25:bridge/usdc")).toBeInTheDocument();
    expect(screen.getByText("$0.25:bridge/usdc")).toBeInTheDocument();
    expect(container).toHaveTextContent(recipient);

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(withdraw.mutate).toHaveBeenCalledOnce();
    expect(bridgeModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("preserves zero-valued backend withdrawal fees in confirmation details", () => {
    const withdraw = createMutation();

    render(
      <BridgeWithdraw
        amount="3.25"
        coin={usdcCoin}
        config={bridgeConfig}
        fee="0"
        recipient="0x4444444444444444444444444444444444444444"
        withdraw={withdraw}
      />,
    );

    expect(screen.getByText("3.25 USDC")).toBeInTheDocument();
    expect(screen.getByText("$0:bridge/usdc")).toBeInTheDocument();
    expect(bridgeModalMocks.getPrice).toHaveBeenCalledWith("0", "bridge/usdc", {
      format: true,
      formatOptions: {
        language: "en-US",
        mask: 1,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(withdraw.mutate).toHaveBeenCalledOnce();
    expect(bridgeModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("closes a bridge withdrawal without submitting when the visible close control is pressed", () => {
    const withdraw = createMutation();

    const { container } = render(
      <BridgeWithdraw
        amount="3.25"
        coin={usdcCoin}
        config={bridgeConfig}
        fee="0.25"
        recipient="0x4444444444444444444444444444444444444444"
        withdraw={withdraw}
      />,
    );

    fireEvent.click(getIconOnlyButton(container));

    expect(withdraw.mutate).not.toHaveBeenCalled();
    expect(bridgeModalMocks.hideModal).toHaveBeenCalledOnce();
  });
});
