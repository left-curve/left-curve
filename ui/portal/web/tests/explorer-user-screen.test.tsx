import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { UserExplorer } from "../src/components/explorer/UserExplorer";

const userExplorerMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  useExplorerUser: vi.fn(),
  useExplorerUserTransactions: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useExplorerUser: userExplorerMocks.useExplorerUser,
  useExplorerUserTransactions: userExplorerMocks.useExplorerUserTransactions,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => userExplorerMocks.navigate,
}));

vi.mock("../src/components/explorer/AssetsTable", () => ({
  AssetsTable: ({
    balances,
    classNames,
  }: {
    balances: Record<string, string>;
    classNames?: { base?: string };
  }) => (
    <div>
      assets-table:
      {Object.entries(balances)
        .map(([denom, amount]) => `${denom}:${amount}`)
        .join("|")}
      :{classNames?.base ?? ""}
    </div>
  ),
}));

vi.mock("../src/components/explorer/TransactionsTable", () => ({
  TransactionsTable: ({
    classNames,
    pagination,
    transactions,
  }: {
    classNames?: { base?: string };
    pagination?: { hasNextPage: boolean; hasPreviousPage: boolean; isLoading: boolean };
    transactions: unknown[];
  }) => (
    <div>
      transactions-table:{transactions.length}:{String(pagination?.hasNextPage)}:
      {String(pagination?.hasPreviousPage)}:{String(pagination?.isLoading)}:{classNames?.base ?? ""}
    </div>
  ),
}));

const firstAccountAddress = "0x6163636f756e7431000000000000000000000000";
const secondAccountAddress = "0x6163636f756e7432000000000000000000000000";
const ethereumKey = "0x6574686b65790000000000000000000000000000";
const passkeyPublicKey = "AQIDBA==";

const userData = {
  accounts: [
    {
      address: firstAccountAddress,
      balance: {
        "bridge/usdc": "1000000",
      },
      balanceUSD: "$1.00",
      index: 0,
      isActive: true,
    },
    {
      address: secondAccountAddress,
      balance: {
        uatom: "2500000",
      },
      balanceUSD: "$2.50",
      index: 2,
      isActive: false,
    },
  ],
  aggregatedBalances: {
    "bridge/usdc": "1000000",
    uatom: "2500000",
  },
  keys: [
    {
      keyHash: "eth-key",
      keyType: "ETHEREUM",
      publicKey: ethereumKey,
    },
    {
      keyHash: "passkey",
      keyType: "SECP256R1",
      publicKey: passkeyPublicKey,
    },
  ],
  totalAccounts: 2,
  totalValue: "$3.50",
  user: {
    index: 42,
    name: "alice",
  },
};

const transactionsResponse = {
  data: [
    {
      hash: "0x7478686173680000000000000000000000000000000000000000000000000000",
    },
  ],
  isLoading: false,
  pagination: {
    goNext: vi.fn(),
    goPrev: vi.fn(),
    hasNextPage: true,
    hasPreviousPage: false,
    isLoading: false,
  },
};

function setUserExplorerData({
  data = userData,
  isLoading = false,
  isNotFound = false,
  transactions = transactionsResponse,
}: {
  data?: typeof userData | null;
  isLoading?: boolean;
  isNotFound?: boolean;
  transactions?: typeof transactionsResponse;
} = {}) {
  userExplorerMocks.useExplorerUser.mockReturnValue({
    data,
    isLoading,
    isNotFound,
  });
  userExplorerMocks.useExplorerUserTransactions.mockReturnValue(transactions);
}

function renderUserExplorer(children: React.ReactNode, username = "alice") {
  render(<UserExplorer username={username}>{children}</UserExplorer>);
}

class MockResizeObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}

function expectTruncatedAccounts(count: number) {
  const starts = screen.getAllByText(firstAccountAddress.slice(0, 4));

  expect(starts).toHaveLength(count);
  for (const start of starts) {
    expect(start.parentElement).toHaveTextContent(firstAccountAddress.slice(0, 4));
    expect(start.parentElement).toHaveTextContent(firstAccountAddress.slice(-4));
  }
}

function copyButtonFor(text: string) {
  const keyRow = screen.getByText(text).closest("div")?.parentElement?.parentElement;

  expect(keyRow).not.toBeNull();

  return within(keyRow as HTMLElement).getByRole("button");
}

describe("user explorer screen", () => {
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", MockResizeObserver);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: vi.fn(),
      },
    });
    setUserExplorerData();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

  it("renders the user header with trader badge and backend aggregate stats", () => {
    renderUserExplorer(<UserExplorer.Header />);

    expect(userExplorerMocks.useExplorerUser).toHaveBeenCalledWith("alice");
    expect(userExplorerMocks.useExplorerUserTransactions).toHaveBeenCalledWith([
      firstAccountAddress,
      secondAccountAddress,
    ]);
    expect(screen.getByAltText("avatar")).toHaveAttribute("src", "/images/avatar.png");
    expect(screen.getByText("alice")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.user.dangoTrader"]())).toHaveClass(
      "bg-account-card-contract",
    );
    expect(screen.getByText(m["explorer.user.stats.userIndex"]())).toBeInTheDocument();
    expect(screen.getByText("#42")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.user.stats.totalAccounts"]())).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getAllByText("$3.50")).toHaveLength(2);
  });

  it("preserves backend user index zero in the rendered user header", () => {
    setUserExplorerData({
      data: {
        ...userData,
        user: {
          ...userData.user,
          index: 0,
          name: "genesis",
        },
      },
    });

    renderUserExplorer(<UserExplorer.Header />, "genesis");

    expect(userExplorerMocks.useExplorerUser).toHaveBeenCalledWith("genesis");
    expect(userExplorerMocks.useExplorerUserTransactions).toHaveBeenCalledWith([
      firstAccountAddress,
      secondAccountAddress,
    ]);
    expect(screen.getByText("genesis")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.user.stats.userIndex"]())).toBeInTheDocument();
    expect(screen.getByText("#0")).toBeInTheDocument();
  });

  it("keeps existing users with zero accounts distinct from not-found users", () => {
    setUserExplorerData({
      data: {
        accounts: [],
        aggregatedBalances: {},
        keys: [],
        totalAccounts: 0,
        totalValue: "$0.00",
        user: {
          index: 88,
          name: "empty-user",
        },
      },
      transactions: {
        ...transactionsResponse,
        data: [],
        pagination: {
          ...transactionsResponse.pagination,
          hasNextPage: false,
        },
      },
    });

    renderUserExplorer(
      <>
        <UserExplorer.Header />
        <UserExplorer.Content />
        <UserExplorer.NotFound />
      </>,
      "empty-user",
    );

    expect(userExplorerMocks.useExplorerUser).toHaveBeenCalledWith("empty-user");
    expect(userExplorerMocks.useExplorerUserTransactions).toHaveBeenCalledWith([]);
    expect(screen.getByText("empty-user")).toBeInTheDocument();
    expect(screen.getByText("#88")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.user.stats.totalAccounts"]())).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();
    expect(screen.getAllByText("$0.00")).toHaveLength(2);
    expect(screen.getByText(m["explorer.user.accounts"]())).toBeInTheDocument();
    expect(screen.queryByText(m["explorer.user.notFound.title"]())).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["explorer.user.tabs.transactions"]() }));

    expect(
      screen.getByText("transactions-table:0:false:false:false:p-0 shadow-none bg-transparent"),
    ).toBeInTheDocument();
  });

  it("renders account stack, defaults to aggregated assets, and navigates to account pages", () => {
    renderUserExplorer(<UserExplorer.Content />);

    expect(screen.getByText(m["explorer.user.accounts"]())).toBeInTheDocument();
    expect(screen.getByText(m["explorer.user.accountNumber"]({ index: 0 }))).toBeInTheDocument();
    expect(screen.getByText(m["explorer.user.accountNumber"]({ index: 2 }))).toBeInTheDocument();
    expectTruncatedAccounts(2);
    expect(screen.getByText("$1.00")).toBeInTheDocument();
    expect(screen.getByText("$2.50")).toBeInTheDocument();
    expect(screen.getByText(m["explorer.user.active"]())).toHaveClass("bg-surface-secondary-blue");
    expect(screen.getByText(m["explorer.user.inactive"]())).toHaveClass("bg-utility-gray-100");
    expect(
      screen.getByText(
        "assets-table:bridge/usdc:1000000|uatom:2500000:p-0 shadow-none bg-transparent",
      ),
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("link", {
        name: new RegExp(m["explorer.user.accountNumber"]({ index: 0 })),
      }),
    );
    expect(userExplorerMocks.navigate).toHaveBeenCalledWith({
      to: `/account/${firstAccountAddress}`,
    });
  });

  it("switches between transaction and key tabs with decoded copy targets", () => {
    renderUserExplorer(<UserExplorer.Content />);

    fireEvent.click(screen.getByRole("button", { name: m["explorer.user.tabs.transactions"]() }));
    expect(
      screen.getByText("transactions-table:1:true:false:false:p-0 shadow-none bg-transparent"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["explorer.user.tabs.keys"]() }));
    expect(screen.getByText(ethereumKey)).toBeInTheDocument();
    expect(screen.getByText("Ethereum Wallet")).toBeInTheDocument();

    fireEvent.pointerUp(copyButtonFor(ethereumKey));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(ethereumKey);

    expect(screen.getByText("0x01020304")).toBeInTheDocument();
    expect(screen.getByText("Passkey")).toBeInTheDocument();

    fireEvent.pointerUp(copyButtonFor("0x01020304"));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith("0x01020304");
  });

  it("renders the not-found branch with the searched username", () => {
    setUserExplorerData({
      data: null,
      isNotFound: true,
      transactions: {
        ...transactionsResponse,
        data: [],
      },
    });

    renderUserExplorer(<UserExplorer.NotFound />, "missing-user");

    expect(screen.getByText(m["explorer.user.notFound.title"]())).toBeInTheDocument();
    expect(
      screen.getByText(m["explorer.user.notFound.description"]({ username: "missing-user" })),
    ).toBeInTheDocument();
    expect(userExplorerMocks.useExplorerUserTransactions).toHaveBeenCalledWith([]);
  });
});
