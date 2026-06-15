import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
} from "./mocks/applets-kit";

import { AuthOptions } from "../src/components/auth/AuthOptions";
import { PasskeyCredential } from "../src/components/auth/PasskeyCredential";
import { SocialCredential } from "../src/components/auth/SocialCredential";
import { UsernamesList } from "../src/components/auth/UsernamesList";
import { createTestQueryClient } from "./utils/query-client";

const authEntryMocks = vi.hoisted(() => ({
  changeSettings: vi.fn(),
  generateURL: vi.fn(),
  loginWithCode: vi.fn(),
  toastError: vi.fn(),
  useConnectors: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useConnectors: authEntryMocks.useConnectors,
}));

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();
  const rendered = render(
    <QueryClientProvider client={queryClient}>{component}</QueryClientProvider>,
  );
  return { queryClient, ...rendered };
}

function getSocialButton(container: HTMLElement, index: number) {
  const button = container.querySelectorAll("button").item(index);
  if (!(button instanceof HTMLButtonElement)) throw new Error("Expected social auth button");
  return button;
}

function getWalletControl(name: string) {
  const label = screen.getByText(name);
  const control = label.closest("[class*='cursor-pointer'], [class*='pointer-events-none']");
  if (!(control instanceof HTMLElement)) throw new Error(`Expected wallet control for ${name}`);
  return control;
}

describe("auth entry components", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      settings: {
        useSessionKey: true,
      },
      changeSettings: authEntryMocks.changeSettings,
      toast: {
        error: authEntryMocks.toastError,
      },
    });
    window.history.pushState({}, "", "/");
    authEntryMocks.generateURL.mockResolvedValue({ url: "#privy-oauth" });
    authEntryMocks.loginWithCode.mockResolvedValue(undefined);
    authEntryMocks.useConnectors.mockReturnValue([
      {
        id: "privy",
        privy: {
          auth: {
            oauth: {
              generateURL: authEntryMocks.generateURL,
              loginWithCode: authEntryMocks.loginWithCode,
            },
          },
        },
        type: "privy",
      },
    ]);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    window.history.pushState({}, "", "/");
  });

  it("requests a Google OAuth URL with the auth callback redirect", async () => {
    const { container } = renderWithQueryClient(<SocialCredential onAuth={vi.fn()} />);

    fireEvent.click(getSocialButton(container, 0));

    await waitFor(() => {
      expect(authEntryMocks.generateURL).toHaveBeenCalledWith(
        "google",
        `${window.location.origin}/?auth_callback=auth`,
      );
    });
    expect(window.location.hash).toBe("#privy-oauth");
  });

  it("completes OAuth callbacks, clears callback params, and runs auth completion", async () => {
    window.history.pushState(
      {},
      "",
      "/?privy_oauth_state=state-1&privy_oauth_code=code-1&privy_oauth_provider=twitter&keep=1",
    );
    const onAuth = vi.fn().mockResolvedValue(undefined);

    renderWithQueryClient(<SocialCredential onAuth={onAuth} />);

    await waitFor(() => {
      expect(authEntryMocks.loginWithCode).toHaveBeenCalledWith(
        "code-1",
        "state-1",
        "twitter",
        undefined,
        "login-or-sign-up",
        {
          embedded: {
            ethereum: {
              createOnLogin: "users-without-wallets",
            },
          },
        },
      );
    });
    expect(onAuth).toHaveBeenCalledOnce();
    expect(window.location.search).toBe("?keep=1");
  });

  it("ignores incomplete OAuth callback params without calling Privy", () => {
    const onAuth = vi.fn().mockResolvedValue(undefined);
    window.history.pushState({}, "", "/?privy_oauth_state=state-1&privy_oauth_code=code-1");

    renderWithQueryClient(<SocialCredential onAuth={onAuth} />);

    expect(authEntryMocks.loginWithCode).not.toHaveBeenCalled();
    expect(onAuth).not.toHaveBeenCalled();
    expect(window.location.search).toBe("?privy_oauth_state=state-1&privy_oauth_code=code-1");
  });

  it("maps OAuth callback failures to the auth toast", async () => {
    window.history.pushState(
      {},
      "",
      "/?privy_oauth_state=state-1&privy_oauth_code=code-1&privy_oauth_provider=google",
    );
    authEntryMocks.loginWithCode.mockRejectedValue(new Error("User does not exist"));

    renderWithQueryClient(<SocialCredential onAuth={vi.fn()} />);

    await waitFor(() => {
      expect(authEntryMocks.toastError).toHaveBeenCalledWith({
        description: m["auth.errors.userNotFound"](),
        title: m["common.error"](),
      });
    });
  });

  it("falls back to the generic auth toast for unknown OAuth callback failures", async () => {
    window.history.pushState(
      {},
      "",
      "/?privy_oauth_state=state-1&privy_oauth_code=code-1&privy_oauth_provider=google",
    );
    authEntryMocks.loginWithCode.mockRejectedValue(new Error("provider rate limited"));
    const onAuth = vi.fn().mockResolvedValue(undefined);

    renderWithQueryClient(<SocialCredential onAuth={onAuth} />);

    await waitFor(() => {
      expect(authEntryMocks.toastError).toHaveBeenCalledWith({
        description: m["auth.errors.authFailed"](),
        title: m["common.error"](),
      });
    });
    expect(onAuth).not.toHaveBeenCalled();
  });

  it("runs the passkey auth callback from the credential button", async () => {
    const onAuth = vi.fn().mockResolvedValue(undefined);
    renderWithQueryClient(<PasskeyCredential label="Use passkey" onAuth={onAuth} />);

    fireEvent.click(screen.getByRole("button", { name: "Use passkey" }));

    await waitFor(() => {
      expect(onAuth).toHaveBeenCalledOnce();
    });
  });

  it("renders only external wallet connectors and disables the others while one is pending", () => {
    const action = vi.fn();
    authEntryMocks.useConnectors.mockReturnValue([
      { icon: "/wallet-a.svg", id: "wallet-a", name: "Wallet A", type: "wallet" },
      { icon: "/wallet-b.svg", id: "wallet-b", name: "Wallet B", type: "eip6963" },
      { icon: "/passkey.svg", id: "passkey", name: "Passkey", type: "passkey" },
      { icon: "/debug.svg", id: "debug", name: "Debug", type: "debug" },
    ]);
    const { container, rerender } = render(<AuthOptions action={action} isPending={false} />);

    expect(getWalletControl("Wallet A")).not.toHaveClass("pointer-events-none");
    expect(getWalletControl("Wallet B")).not.toHaveClass("pointer-events-none");
    expect(screen.queryByText("Passkey")).not.toBeInTheDocument();
    expect(screen.queryByText("Debug")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("Wallet A"));

    expect(action).toHaveBeenCalledWith("wallet-a");

    rerender(<AuthOptions action={action} isPending />);

    expect(container.querySelector(".animate-spinner-ease-spin")).toBeInTheDocument();
    expect(getWalletControl("Wallet B")).toHaveClass("pointer-events-none");
  });

  it("shows the no-wallet message when there are no external connectors", () => {
    authEntryMocks.useConnectors.mockReturnValue([
      { id: "privy", name: "Privy", type: "privy" },
      { id: "session", name: "Session", type: "session" },
    ]);

    render(<AuthOptions action={vi.fn()} isPending={false} />);

    expect(screen.getByText(m["common.notWalletDetected"]())).toBeInTheDocument();
  });

  it("selects usernames by user index", () => {
    const onUserSelection = vi.fn();
    render(
      <UsernamesList
        users={[
          { index: 7, name: "alice" },
          { index: 9, name: "bob" },
        ]}
        onUserSelection={onUserSelection}
      />,
    );

    fireEvent.click(screen.getByText("bob"));

    expect(onUserSelection).toHaveBeenCalledWith(9);
  });
});
