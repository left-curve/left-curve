import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";

import { DestinationWallet } from "../src/components/modals/DestinationWallet";

const destinationWalletMocks = vi.hoisted(() => ({
  connectors: [] as Array<{
    getProvider: () => Promise<{
      request: (params: { method: string }) => Promise<string[]>;
    }>;
    icon?: string;
    name: string;
    type: string;
    uid: string;
  }>,
  hideModal: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useConnectors: () => destinationWalletMocks.connectors,
}));

function connector({
  accounts = ["0x1111111111111111111111111111111111111111"],
  icon,
  name,
  request = vi.fn(async () => accounts),
  type = "injected",
  uid,
}: {
  accounts?: string[];
  icon?: string;
  name: string;
  request?: ReturnType<typeof vi.fn>;
  type?: string;
  uid: string;
}) {
  return {
    getProvider: vi.fn(async () => ({
      request,
    })),
    icon,
    name,
    type,
    uid,
  };
}

describe("destination wallet modal", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: destinationWalletMocks.hideModal,
    });
    const browserWalletRequest = vi.fn(async () => ["0x1111111111111111111111111111111111111111"]);
    destinationWalletMocks.connectors = [
      connector({
        icon: "/wallet.svg",
        name: "Browser Wallet",
        request: browserWalletRequest,
        uid: "browser-wallet",
      }),
      connector({ name: "Passkey", type: "passkey", uid: "passkey" }),
      connector({ name: "Session", type: "session", uid: "session" }),
      connector({ name: "Privy", type: "privy", uid: "privy" }),
    ];
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("filters unsupported connectors and uses the selected wallet account", async () => {
    const onAddressSet = vi.fn();
    const wallet = destinationWalletMocks.connectors[0];

    render(<DestinationWallet network="ethereum" onAddressSet={onAddressSet} />);

    expect(screen.getByText(m["bridge.destinationWallet"]())).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Browser Wallet/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Passkey" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Session" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Privy" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Browser Wallet/ }));

    await waitFor(() => {
      expect(onAddressSet).toHaveBeenCalledWith(
        "0x1111111111111111111111111111111111111111",
        "Browser Wallet",
        "/wallet.svg",
      );
    });
    expect(wallet.getProvider).toHaveBeenCalledOnce();
    const provider = await wallet.getProvider.mock.results[0].value;
    expect(provider.request).toHaveBeenCalledWith({ method: "eth_requestAccounts" });
    expect(destinationWalletMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("keeps the modal open when the wallet provider rejects account access", async () => {
    destinationWalletMocks.connectors = [
      connector({
        name: "Rejecting Wallet",
        request: vi.fn(async () => {
          throw new Error("user rejected");
        }),
        uid: "rejecting-wallet",
      }),
    ];
    const onAddressSet = vi.fn();

    render(<DestinationWallet network="ethereum" onAddressSet={onAddressSet} />);

    fireEvent.click(screen.getByRole("button", { name: "Rejecting Wallet" }));

    await waitFor(() => {
      expect(destinationWalletMocks.connectors[0].getProvider).toHaveBeenCalledOnce();
    });
    expect(onAddressSet).not.toHaveBeenCalled();
    expect(destinationWalletMocks.hideModal).not.toHaveBeenCalled();
  });

  it("keeps the modal open when the wallet provider cannot be created", async () => {
    destinationWalletMocks.connectors = [
      {
        getProvider: vi.fn(async () => {
          throw new Error("provider unavailable");
        }),
        name: "Unavailable Wallet",
        type: "injected",
        uid: "unavailable-wallet",
      },
    ];
    const onAddressSet = vi.fn();

    render(<DestinationWallet network="ethereum" onAddressSet={onAddressSet} />);

    fireEvent.click(screen.getByRole("button", { name: "Unavailable Wallet" }));

    await waitFor(() => {
      expect(destinationWalletMocks.connectors[0].getProvider).toHaveBeenCalledOnce();
    });
    expect(onAddressSet).not.toHaveBeenCalled();
    expect(destinationWalletMocks.hideModal).not.toHaveBeenCalled();
    expect(screen.getByText(m["bridge.destinationWallet"]())).toBeInTheDocument();
  });

  it("keeps the modal open when a wallet provider returns no accounts", async () => {
    destinationWalletMocks.connectors = [
      connector({
        accounts: [],
        name: "Empty Wallet",
        uid: "empty-wallet",
      }),
    ];
    const onAddressSet = vi.fn();

    render(<DestinationWallet network="ethereum" onAddressSet={onAddressSet} />);

    fireEvent.click(screen.getByRole("button", { name: "Empty Wallet" }));

    await waitFor(() => {
      expect(destinationWalletMocks.connectors[0].getProvider).toHaveBeenCalledOnce();
    });
    const provider = await destinationWalletMocks.connectors[0].getProvider.mock.results[0].value;
    expect(provider.request).toHaveBeenCalledWith({ method: "eth_requestAccounts" });
    expect(onAddressSet).not.toHaveBeenCalled();
    expect(destinationWalletMocks.hideModal).not.toHaveBeenCalled();
  });

  it("requires risk acknowledgement and a valid manual 0x address", () => {
    const onAddressSet = vi.fn();

    render(<DestinationWallet network="ethereum" onAddressSet={onAddressSet} />);

    fireEvent.click(screen.getByRole("button", { name: m["bridge.enterAddressManually"]() }));

    expect(screen.getByText(m["bridge.riskOfFundLoss"]())).toBeInTheDocument();
    expect(screen.getByText(m["bridge.riskOfFundLossDescription"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["bridge.iUnderstandTheRisk"]() }));

    expect(screen.getByText(m["bridge.withdrawAddress"]())).toBeInTheDocument();

    const input = screen.getByRole("textbox");
    const confirmButton = screen.getByRole("button", { name: m["bridge.confirm"]() });

    expect(screen.getByText(m["bridge.exchangeWarning"]())).toBeInTheDocument();
    expect(confirmButton).toBeDisabled();

    fireEvent.change(input, {
      target: { value: "not-a-wallet" },
    });

    expect(input).toHaveValue("");
    expect(confirmButton).toBeDisabled();
    expect(onAddressSet).not.toHaveBeenCalled();

    fireEvent.change(input, {
      target: { value: "0x123" },
    });

    expect(confirmButton).toBeDisabled();
    expect(onAddressSet).not.toHaveBeenCalled();

    fireEvent.change(input, {
      target: { value: "0x2222222222222222222222222222222222222222" },
    });
    fireEvent.click(confirmButton);

    expect(onAddressSet).toHaveBeenCalledWith("0x2222222222222222222222222222222222222222");
    expect(destinationWalletMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("closes the manual address flow without setting a recipient", () => {
    const onAddressSet = vi.fn();

    const { container } = render(
      <DestinationWallet network="ethereum" onAddressSet={onAddressSet} />,
    );

    fireEvent.click(screen.getByRole("button", { name: m["bridge.enterAddressManually"]() }));
    fireEvent.click(screen.getByRole("button", { name: m["bridge.iUnderstandTheRisk"]() }));

    const closeButton = container.querySelector("button.absolute");
    if (!closeButton) throw new Error("Expected destination wallet close button");

    fireEvent.click(closeButton);

    expect(onAddressSet).not.toHaveBeenCalled();
    expect(destinationWalletMocks.hideModal).toHaveBeenCalledOnce();
  });
});
