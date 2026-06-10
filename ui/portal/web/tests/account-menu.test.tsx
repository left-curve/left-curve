import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { Account } from "@left-curve/types";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
  setAppletsKitUseBodyScrollLockFactory,
  setAppletsKitUseClickAwayFactory,
  setAppletsKitUseHeaderHeight,
  setAppletsKitUseMediaQuery,
} from "./mocks/applets-kit";

import { Modals } from "@left-curve/applets-kit";

import { AccountMenu } from "../src/components/foundation/AccountMenu";

const accountMenuMocks = vi.hoisted(() => ({
  changeAccount: vi.fn(),
  deleteSessionKey: vi.fn(),
  disconnect: vi.fn(),
  markAllSeen: vi.fn(),
  navigate: vi.fn(),
  setSidebarVisibility: vi.fn(),
  showModal: vi.fn(),
}));

const activeAccount: Account = {
  address: "0x6163746976650000000000000000000000000000",
  index: 1,
  owner: 7,
};

const secondAccount: Account = {
  address: "0x7365636f6e640000000000000000000000000000",
  index: 2,
  owner: 7,
};

const thirdAccount: Account = {
  address: "0x7468697264000000000000000000000000000000",
  index: 3,
  owner: 7,
};

class TestResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

vi.stubGlobal("ResizeObserver", TestResizeObserver);

vi.mock("framer-motion", () => ({
  AnimatePresence: ({ children }: React.PropsWithChildren) => <>{children}</>,
  motion: {
    div: ({
      animate: _animate,
      children,
      exit: _exit,
      initial: _initial,
      transition: _transition,
      ...props
    }: React.HTMLAttributes<HTMLDivElement> & {
      animate?: unknown;
      exit?: unknown;
      initial?: unknown;
      transition?: unknown;
    }) => <div {...props}>{children}</div>,
    span: ({
      animate: _animate,
      children,
      exit: _exit,
      initial: _initial,
      transition: _transition,
      ...props
    }: React.HTMLAttributes<HTMLSpanElement> & {
      animate?: unknown;
      exit?: unknown;
      initial?: unknown;
      transition?: unknown;
    }) => <span {...props}>{children}</span>,
  },
}));

vi.mock("react-modal-sheet", () => ({
  Sheet: Object.assign(
    ({ children, isOpen }: React.PropsWithChildren<{ isOpen: boolean }>) =>
      isOpen ? <div>{children}</div> : null,
    {
      Backdrop: ({ onTap }: { onTap?: () => void }) => (
        <button data-testid="sheet-backdrop" onClick={onTap} type="button" />
      ),
      Container: ({ children }: React.PropsWithChildren) => <div>{children}</div>,
      Content: ({ children }: React.PropsWithChildren) => <div>{children}</div>,
      Header: () => <div data-testid="sheet-header" />,
    },
  ),
}));

vi.mock("../src/components/foundation/AssetCard", () => {
  const AssetCard = ({ coin }: { coin: { amount: string; denom: string } }) => (
    <div data-testid="asset-card">
      {coin.denom}:{coin.amount}
    </div>
  );

  return {
    AssetCard: Object.assign(AssetCard, {
      Perp: ({ amount }: { amount: string }) => <div data-testid="perp-card">perp:{amount}</div>,
      Vault: ({ shares, usdValue }: { shares?: string; usdValue?: string }) => (
        <div data-testid="vault-card">
          vault:{shares ?? "0"}:{usdValue ?? "0"}
        </div>
      ),
    }),
  };
});

vi.mock("../src/components/foundation/CountBadge", () => ({
  CountBadge: ({ count }: { count: number }) => <span data-testid="count-badge">{count}</span>,
}));

vi.mock("../src/components/foundation/EmptyPlaceholder", () => ({
  EmptyPlaceholder: ({ component }: { component: React.ReactNode }) => <div>{component}</div>,
}));

vi.mock("../src/components/activities/Activities", () => ({
  Activities: () => <div data-testid="activities-list" />,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => accountMenuMocks.navigate,
  useRouter: () => ({
    history: {
      go: vi.fn(),
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    account: activeAccount,
    accounts: [thirdAccount, activeAccount, secondAccount],
    changeAccount: accountMenuMocks.changeAccount,
    connector: {
      disconnect: accountMenuMocks.disconnect,
    },
    isUserActive: true,
  }),
  useActivities: () => ({
    markAllSeen: accountMenuMocks.markAllSeen,
    unseenCount: 4,
  }),
  useBalances: () => ({
    data: {
      "bridge/usdc": "2500000",
      uatom: "1000000",
    },
  }),
  usePerpsUserState: (selector: (state: { userState: { margin: string } }) => unknown) =>
    selector({ userState: { margin: "1250000" } }),
  usePerpsUserStateExtended: () => "3.5",
  usePerpsVaultUserShares: () => ({
    userSharesValue: "4.25",
    userVaultShares: "2",
  }),
  usePrices: () => ({
    calculateBalance: vi.fn((_coins: unknown, options?: { format?: boolean }) =>
      options?.format === false ? "7.75" : "$7.75",
    ),
  }),
  useSessionKey: () => ({
    deleteSessionKey: accountMenuMocks.deleteSessionKey,
  }),
}));

function getTopActionButtons() {
  const fundButton = screen.getByRole("button", { name: m["common.fund"]() });
  const row = fundButton.parentElement;
  if (!row) throw new Error("Expected fund button to be inside the top action row");

  return within(row).getAllByRole("button");
}

function getByTextContent(text: string) {
  return screen.getByText((_content, element) => {
    if (!element?.textContent?.includes(text)) return false;
    return Array.from(element.children).every((child) => !child.textContent?.includes(text));
  });
}

describe("account menu", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      isSidebarVisible: true,
      modal: {},
      setSidebarVisibility: accountMenuMocks.setSidebarVisibility,
      settings: {
        formatNumberOptions: {},
      },
      showModal: accountMenuMocks.showModal,
    });
    setAppletsKitUseBodyScrollLockFactory(() => undefined);
    setAppletsKitUseClickAwayFactory(() => undefined);
    setAppletsKitUseHeaderHeight(72);
    setAppletsKitUseMediaQuery({
      isLg: true,
      isMd: true,
    });
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("routes account actions, opens QR connect, marks activities seen, and logs out cleanly", () => {
    render(<AccountMenu />);

    expect(
      screen.getByText(`${m["common.account"]()} #${activeAccount.index}`),
    ).toBeInTheDocument();
    expect(screen.getByText("$15.50")).toBeInTheDocument();
    expect(getByTextContent("bridge/usdc:2500000")).toBeInTheDocument();
    expect(getByTextContent("uatom:1000000")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["common.fund"]() }));

    expect(accountMenuMocks.navigate).toHaveBeenCalledWith({ to: "/bridge" });
    expect(accountMenuMocks.setSidebarVisibility).toHaveBeenCalledWith(false);

    vi.clearAllMocks();

    fireEvent.click(screen.getByRole("button", { name: m["accountMenu.spotPerp"]() }));

    expect(accountMenuMocks.navigate).toHaveBeenCalledWith({
      search: { action: "spot-perp" },
      to: "/transfer",
    });
    expect(accountMenuMocks.setSidebarVisibility).toHaveBeenCalledWith(false);

    vi.clearAllMocks();

    const [, , qrConnectButton] = getTopActionButtons();
    fireEvent.click(qrConnectButton);

    expect(accountMenuMocks.showModal).toHaveBeenCalledWith(Modals.QRConnect);

    vi.clearAllMocks();

    fireEvent.click(screen.getByRole("button", { name: new RegExp(m["activities.title"]()) }));

    expect(accountMenuMocks.markAllSeen).toHaveBeenCalledOnce();
    expect(screen.getByTestId("activities-list")).toBeInTheDocument();

    vi.clearAllMocks();

    const [, , , logoutButton] = getTopActionButtons();
    fireEvent.click(logoutButton);

    expect(accountMenuMocks.setSidebarVisibility).toHaveBeenCalledWith(false);
    expect(accountMenuMocks.disconnect).toHaveBeenCalledOnce();
    expect(accountMenuMocks.deleteSessionKey).toHaveBeenCalledOnce();
  });

  it("opens the sorted account selector, switches account, and routes to account creation", () => {
    render(<AccountMenu />);

    fireEvent.click(screen.getByRole("button", { name: m["common.switch"]() }));

    const selector = screen.getByText(m["accountMenu.accounts.addAccount"]()).closest("div");
    if (!selector) throw new Error("Expected account selector controls");

    const previews = screen
      .getAllByText(new RegExp(`^${m["common.account"]()} #`))
      .map((node) => node.textContent);
    expect(previews).toEqual([
      `${m["common.account"]()} #${activeAccount.index}`,
      `${m["common.account"]()} #${secondAccount.index}`,
      `${m["common.account"]()} #${thirdAccount.index}`,
    ]);
    expect(screen.getByText(activeAccount.address.slice(0, 4)).closest("p")).toHaveTextContent(
      activeAccount.address.slice(-4),
    );

    fireEvent.click(screen.getByText(`${m["common.account"]()} #${secondAccount.index}`));

    expect(accountMenuMocks.changeAccount).toHaveBeenCalledWith(secondAccount.address);

    vi.clearAllMocks();

    fireEvent.click(
      within(selector).getByRole("button", { name: m["accountMenu.accounts.addAccount"]() }),
    );

    expect(accountMenuMocks.setSidebarVisibility).toHaveBeenCalledWith(false);
    expect(accountMenuMocks.navigate).toHaveBeenCalledWith({ to: "/account/create" });

    vi.clearAllMocks();

    fireEvent.click(screen.getByRole("button", { name: m["common.back"]() }));

    expect(screen.getByRole("button", { name: m["accountMenu.spotPerp"]() })).toBeInTheDocument();
  });
});
