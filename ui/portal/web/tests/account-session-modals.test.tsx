import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";

import { DEFAULT_SESSION_EXPIRATION } from "../constants.config";
import { AddressWarning } from "../src/components/modals/AddressWarning";
import { Authenticate } from "../src/components/modals/Authenticate";
import { ConfirmAccount } from "../src/components/modals/ConfirmAccount";
import { RenewSession } from "../src/components/modals/RenewSession";
import { SignupReminder } from "../src/components/modals/SignupReminder";
import { WalletSelector } from "../src/components/modals/WalletSelector";

const accountSessionModalMocks = vi.hoisted(() => ({
  changeAccount: vi.fn(),
  connectorDisconnect: vi.fn(),
  createSessionKey: vi.fn(),
  getPrice: vi.fn(),
  hideModal: vi.fn(),
  navigate: vi.fn(),
  refreshAccounts: vi.fn(),
  useAccount: vi.fn(),
  useSessionKey: vi.fn(),
}));

vi.mock("@left-curve/utils", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/utils")>();

  return {
    ...actual,
    wait: vi.fn(() => Promise.resolve()),
  };
});

vi.mock("../src/components/auth/AuthOptions", () => ({
  AuthOptions: ({ action, isPending }: { action: (id: string) => void; isPending: boolean }) => (
    <div>
      <button disabled={isPending} onClick={() => action("wallet-a")} type="button">
        Wallet A
      </button>
      <button disabled={isPending} onClick={() => action("wallet-b")} type="button">
        Wallet B
      </button>
    </div>
  ),
}));

vi.mock("../src/components/auth/AuthFlow", () => ({
  AuthFlow: ({ onFinish, referrer }: { onFinish: () => void; referrer?: number }) => (
    <section data-referrer={referrer} data-testid="auth-flow">
      <button onClick={onFinish} type="button">
        finish auth
      </button>
    </section>
  ),
}));

vi.mock("@left-curve/foundation", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@left-curve/foundation")>()),
  useApp: () => ({
    settings: {
      formatNumberOptions: {
        currency: "USD",
        language: "en-US",
        mask: 2,
      },
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: accountSessionModalMocks.useAccount,
  useConfig: () => ({
    coins: {
      byDenom: {
        "bridge/usdc": {
          decimals: 6,
          denom: "bridge/usdc",
          logoURI: "/usdc.png",
          symbol: "USDC",
        },
      },
    },
  }),
  usePrices: () => ({
    getPrice: accountSessionModalMocks.getPrice,
  }),
  useSessionKey: accountSessionModalMocks.useSessionKey,
}));

describe("account and session modals", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: accountSessionModalMocks.hideModal,
      navigate: accountSessionModalMocks.navigate,
    });
    accountSessionModalMocks.getPrice.mockImplementation((amount: string) => Number(amount));
    accountSessionModalMocks.refreshAccounts.mockResolvedValue(undefined);
    accountSessionModalMocks.useAccount.mockReturnValue({
      changeAccount: accountSessionModalMocks.changeAccount,
      connector: {
        disconnect: accountSessionModalMocks.connectorDisconnect,
      },
      refreshAccounts: accountSessionModalMocks.refreshAccounts,
    });
    accountSessionModalMocks.useSessionKey.mockReturnValue({
      createSessionKey: accountSessionModalMocks.createSessionKey,
      session: undefined,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.restoreAllMocks();
  });

  it("refreshes accounts, routes home, and activates the new account after confirmation", async () => {
    const navigate = vi.fn();
    const accountAddress = "0x6e65776163636f756e7400000000000000000000";

    render(
      <ConfirmAccount
        accountAddress={accountAddress}
        accountName="Account #3"
        amount="2500000"
        denom="bridge/usdc"
        navigate={navigate as never}
      />,
    );

    expect(screen.getByText(m["modals.accountCreation.title"]())).toBeInTheDocument();
    expect(screen.getByText("Account #3")).toBeInTheDocument();
    expect(screen.getByText("2.5 USDC")).toBeInTheDocument();
    expect(accountSessionModalMocks.getPrice).toHaveBeenCalledWith("2.5", "bridge/usdc");

    fireEvent.click(screen.getByRole("button", { name: m["modals.accountCreation.getStarted"]() }));

    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith({ to: "/" });
    });
    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(accountSessionModalMocks.refreshAccounts).toHaveBeenCalledOnce();
    expect(accountSessionModalMocks.changeAccount).toHaveBeenCalledWith(accountAddress);
    expect(accountSessionModalMocks.hideModal.mock.invocationCallOrder[0]).toBeLessThan(
      accountSessionModalMocks.refreshAccounts.mock.invocationCallOrder[0],
    );
    expect(accountSessionModalMocks.refreshAccounts.mock.invocationCallOrder[0]).toBeLessThan(
      navigate.mock.invocationCallOrder[0],
    );
    expect(navigate.mock.invocationCallOrder[0]).toBeLessThan(
      accountSessionModalMocks.changeAccount.mock.invocationCallOrder[0],
    );
  });

  it("waits for backend account refresh before navigating or activating the account", async () => {
    const navigate = vi.fn();
    const accountAddress = "0x726566726573682d676174656400000000000000";
    let resolveRefresh!: () => void;
    const refreshPromise = new Promise<void>((resolve) => {
      resolveRefresh = resolve;
    });
    accountSessionModalMocks.refreshAccounts.mockReturnValueOnce(refreshPromise);

    render(
      <ConfirmAccount
        accountAddress={accountAddress}
        accountName="Account #4"
        amount="1000000"
        denom="bridge/usdc"
        navigate={navigate as never}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: m["modals.accountCreation.getStarted"]() }));

    await waitFor(() => {
      expect(accountSessionModalMocks.refreshAccounts).toHaveBeenCalledOnce();
    });
    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(navigate).not.toHaveBeenCalled();
    expect(accountSessionModalMocks.changeAccount).not.toHaveBeenCalled();

    await act(async () => {
      resolveRefresh();
      await refreshPromise;
    });

    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith({ to: "/" });
    });
    expect(accountSessionModalMocks.changeAccount).toHaveBeenCalledWith(accountAddress);
  });

  it("preserves backend account index zero in account confirmation details", async () => {
    const navigate = vi.fn();
    const accountAddress = "0x67656e6573697300000000000000000000000000";

    render(
      <ConfirmAccount
        accountAddress={accountAddress}
        accountName="Account #0"
        amount="0"
        denom="bridge/usdc"
        navigate={navigate as never}
      />,
    );

    expect(screen.getByText("Account #0")).toBeInTheDocument();
    expect(screen.getByText("0 USDC")).toBeInTheDocument();
    expect(accountSessionModalMocks.getPrice).toHaveBeenCalledWith("0", "bridge/usdc");

    fireEvent.click(screen.getByRole("button", { name: m["modals.accountCreation.getStarted"]() }));

    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith({ to: "/" });
    });
    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(accountSessionModalMocks.refreshAccounts).toHaveBeenCalledOnce();
    expect(accountSessionModalMocks.changeAccount).toHaveBeenCalledWith(accountAddress);
  });

  it("closes account confirmation without activating the new account", () => {
    const navigate = vi.fn();

    const { container } = render(
      <ConfirmAccount
        accountAddress="0x6e65776163636f756e7400000000000000000000"
        accountName="Account #3"
        amount="2500000"
        denom="bridge/usdc"
        navigate={navigate as never}
      />,
    );

    const closeButton = container.querySelector("button.absolute");
    if (!closeButton) throw new Error("Expected account confirmation close button");

    fireEvent.click(closeButton);

    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(accountSessionModalMocks.refreshAccounts).not.toHaveBeenCalled();
    expect(navigate).not.toHaveBeenCalled();
    expect(accountSessionModalMocks.changeAccount).not.toHaveBeenCalled();
  });

  it("selects a wallet connector and rejects when closed", () => {
    const onWalletSelect = vi.fn();
    const onReject = vi.fn();
    const modalRef = createRef<{ triggerOnClose: () => void }>();

    render(<WalletSelector ref={modalRef} onReject={onReject} onWalletSelect={onWalletSelect} />);

    expect(screen.getByText(m["modals.walletSelector.title"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Wallet A" }));

    expect(onWalletSelect).toHaveBeenCalledWith("wallet-a");
    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(onReject).not.toHaveBeenCalled();

    modalRef.current?.triggerOnClose();
    expect(onReject).toHaveBeenCalledOnce();
  });

  it("rejects wallet selection from the close button", () => {
    const onWalletSelect = vi.fn();
    const onReject = vi.fn();

    render(<WalletSelector onReject={onReject} onWalletSelect={onWalletSelect} />);

    const closeButton = screen
      .getAllByRole("button")
      .find((button) => !["Wallet A", "Wallet B"].includes(button.textContent ?? ""));

    if (!closeButton) throw new Error("Expected wallet selector close button");

    fireEvent.click(closeButton);

    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(onReject).toHaveBeenCalledOnce();
    expect(onWalletSelect).not.toHaveBeenCalled();
  });

  it("passes modal referrers into auth flow and hides when auth finishes or closes", () => {
    const { container } = render(<Authenticate referrer={42} />);

    expect(screen.getByTestId("auth-flow")).toHaveAttribute("data-referrer", "42");

    fireEvent.click(screen.getByRole("button", { name: "finish auth" }));

    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();

    cleanup();
    vi.clearAllMocks();

    const { container: modalContainer } = render(<Authenticate />);
    const closeButton = modalContainer.querySelector("button.absolute");

    if (!closeButton) throw new Error("Expected authenticate close button");

    fireEvent.click(closeButton);

    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(screen.getByTestId("auth-flow")).not.toHaveAttribute("data-referrer");
    expect(container).toBeEmptyDOMElement();
  });

  it("renews the session with the default expiration and logs out on request", () => {
    const now = 1_700_000_000_000;
    vi.spyOn(Date, "now").mockReturnValue(now);

    render(<RenewSession />);

    fireEvent.click(screen.getByRole("button", { name: m["common.signin"]() }));

    expect(accountSessionModalMocks.createSessionKey).toHaveBeenCalledWith({
      expireAt: now + DEFAULT_SESSION_EXPIRATION,
    });

    fireEvent.click(screen.getByRole("button", { name: m["modals.renewSession.stayLogout"]() }));

    expect(accountSessionModalMocks.connectorDisconnect).toHaveBeenCalledOnce();
    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("hides the renew-session modal when a valid session already exists", async () => {
    const now = 1_700_000_000_000;
    vi.spyOn(Date, "now").mockReturnValue(now);
    accountSessionModalMocks.useSessionKey.mockReturnValue({
      createSessionKey: accountSessionModalMocks.createSessionKey,
      session: {
        sessionInfo: {
          expireAt: String(Math.floor(now / 1000) + 60),
        },
      },
    });

    render(<RenewSession />);

    await waitFor(() => {
      expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    });
    expect(accountSessionModalMocks.createSessionKey).not.toHaveBeenCalled();
    expect(accountSessionModalMocks.connectorDisconnect).not.toHaveBeenCalled();
  });

  it("keeps renew-session actions available when the stored session is expired", () => {
    const now = 1_700_000_000_000;
    vi.spyOn(Date, "now").mockReturnValue(now);
    accountSessionModalMocks.useSessionKey.mockReturnValue({
      createSessionKey: accountSessionModalMocks.createSessionKey,
      session: {
        sessionInfo: {
          expireAt: String(Math.floor(now / 1000) - 1),
        },
      },
    });

    render(<RenewSession />);

    expect(screen.getByText(m["modals.renewSession.title"]())).toBeInTheDocument();
    expect(screen.getByRole("button", { name: m["common.signin"]() })).toBeEnabled();
    expect(
      screen.getByRole("button", { name: m["modals.renewSession.stayLogout"]() }),
    ).toBeEnabled();
    expect(accountSessionModalMocks.hideModal).not.toHaveBeenCalled();
    expect(accountSessionModalMocks.createSessionKey).not.toHaveBeenCalled();
    expect(accountSessionModalMocks.connectorDisconnect).not.toHaveBeenCalled();
  });

  it("routes address-warning users to the bridge or closes the warning", () => {
    render(<AddressWarning />);

    expect(screen.getByText(m["accountCard.addressWarning.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["accountCard.addressWarning.bullet1"]())).toBeInTheDocument();
    expect(screen.getByText(m["accountCard.addressWarning.bullet2"]())).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: m["accountCard.addressWarning.descriptionLink"]() }),
    );

    expect(accountSessionModalMocks.navigate).toHaveBeenCalledWith("/bridge");
    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();

    cleanup();
    vi.clearAllMocks();
    render(<AddressWarning />);

    fireEvent.click(screen.getByRole("button", { name: m["accountCard.addressWarning.button"]() }));

    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(accountSessionModalMocks.navigate).not.toHaveBeenCalled();
  });

  it("renders the signup reminder quest link and closes from the modal control", () => {
    const { container } = render(<SignupReminder />);

    const questLink = container.querySelector<HTMLAnchorElement>(
      'a[href="https://app.galxe.com/quest/dango/GCNAXt8Tqv"]',
    );
    expect(questLink).not.toBeNull();
    expect(questLink).toHaveAttribute("target", "_blank");
    expect(questLink).toHaveAttribute("rel", "noreferrer");
    expect(screen.getByAltText("dango logo")).toHaveAttribute("src", "/favicon.svg");

    const closeButton = container.querySelector("button.absolute");
    if (!closeButton) throw new Error("Expected signup reminder close button");

    fireEvent.click(closeButton);

    expect(accountSessionModalMocks.hideModal).toHaveBeenCalledOnce();
    expect(accountSessionModalMocks.navigate).not.toHaveBeenCalled();
  });
});
