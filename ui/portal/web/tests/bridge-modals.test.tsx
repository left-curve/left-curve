import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";

import { BridgeDeposit } from "../src/components/modals/BridgeDeposit";
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

async function renderAndFlushEffects(component: React.ReactNode) {
  let rendered!: ReturnType<typeof render>;
  await act(async () => {
    rendered = render(component);
    await Promise.resolve();
    await Promise.resolve();
  });
  return rendered;
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

  it("runs approval before deposit, resets the bridge form, and renders the completed tx link", async () => {
    const allowanceMutation = createMutation();
    const deposit = createMutation();
    const reset = vi.fn();
    allowanceMutation.mutateAsync.mockResolvedValue(undefined);
    deposit.mutateAsync.mockResolvedValue("0xdeposit");
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);

    const { container } = await renderAndFlushEffects(
      <BridgeDeposit
        allowanceMutation={allowanceMutation}
        amount="12.5"
        coin={usdcCoin}
        config={bridgeConfig}
        deposit={deposit}
        requiresAllowance
        reset={reset}
      />,
    );

    expect(screen.getByRole("heading", { name: m["bridge.deposit.title"]() })).toBeInTheDocument();
    expect(screen.getByText("12.5 USDC")).toBeInTheDocument();
    expect(screen.getByText("$12.5:bridge/usdc")).toBeInTheDocument();

    await waitFor(() => {
      expect(allowanceMutation.mutateAsync).toHaveBeenCalledOnce();
    });
    expect(allowanceMutation.mutateAsync.mock.invocationCallOrder[0]).toBeLessThan(
      deposit.mutateAsync.mock.invocationCallOrder[0],
    );
    expect(deposit.mutateAsync).toHaveBeenCalledOnce();
    expect(reset).toHaveBeenCalledOnce();
    expect(bridgeModalMocks.hideModal).not.toHaveBeenCalled();

    const completedLinkButton = container.querySelector("button.p-0");
    if (!(completedLinkButton instanceof HTMLButtonElement)) {
      throw new Error("Expected completed transaction link button");
    }

    fireEvent.click(completedLinkButton);
    expect(openSpy).toHaveBeenCalledWith("https://explorer.example/tx/0xdeposit", "_blank");

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(bridgeModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("completes direct deposits without allowance and renders the backend tx link", async () => {
    const allowanceMutation = createMutation();
    const deposit = createMutation();
    const reset = vi.fn();
    deposit.mutateAsync.mockResolvedValue("0xdirectdeposit");
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);

    const { container } = await renderAndFlushEffects(
      <BridgeDeposit
        allowanceMutation={allowanceMutation}
        amount="4"
        coin={usdcCoin}
        config={bridgeConfig}
        deposit={deposit}
        requiresAllowance={false}
        reset={reset}
      />,
    );

    await waitFor(() => {
      expect(deposit.mutateAsync).toHaveBeenCalledOnce();
    });
    expect(allowanceMutation.mutateAsync).not.toHaveBeenCalled();
    expect(reset).toHaveBeenCalledOnce();
    expect(bridgeModalMocks.hideModal).not.toHaveBeenCalled();

    const completedLinkButton = container.querySelector("button.p-0");
    if (!(completedLinkButton instanceof HTMLButtonElement)) {
      throw new Error("Expected completed transaction link button");
    }

    fireEvent.click(completedLinkButton);
    expect(openSpy).toHaveBeenCalledWith("https://explorer.example/tx/0xdirectdeposit", "_blank");

    fireEvent.click(screen.getByRole("button", { name: m["common.confirm"]() }));

    expect(bridgeModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("skips allowance when it is not required and closes the modal on deposit failure", async () => {
    const allowanceMutation = createMutation();
    const deposit = createMutation();
    const reset = vi.fn();
    deposit.mutateAsync.mockRejectedValue(new Error("wallet rejected"));

    await renderAndFlushEffects(
      <BridgeDeposit
        allowanceMutation={allowanceMutation}
        amount="4"
        coin={usdcCoin}
        config={bridgeConfig}
        deposit={deposit}
        requiresAllowance={false}
        reset={reset}
      />,
    );

    await waitFor(() => {
      expect(deposit.mutateAsync).toHaveBeenCalledOnce();
    });
    expect(allowanceMutation.mutateAsync).not.toHaveBeenCalled();
    expect(reset).not.toHaveBeenCalled();
    expect(bridgeModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("does not submit the deposit when the required allowance approval fails", async () => {
    const allowanceMutation = createMutation();
    const deposit = createMutation();
    const reset = vi.fn();
    allowanceMutation.mutateAsync.mockRejectedValue(new Error("approval rejected"));

    await renderAndFlushEffects(
      <BridgeDeposit
        allowanceMutation={allowanceMutation}
        amount="4"
        coin={usdcCoin}
        config={bridgeConfig}
        deposit={deposit}
        requiresAllowance
        reset={reset}
      />,
    );

    await waitFor(() => {
      expect(allowanceMutation.mutateAsync).toHaveBeenCalledOnce();
    });
    expect(deposit.mutateAsync).not.toHaveBeenCalled();
    expect(reset).not.toHaveBeenCalled();
    expect(bridgeModalMocks.hideModal).toHaveBeenCalledOnce();
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
