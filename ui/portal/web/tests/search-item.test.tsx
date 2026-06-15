import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type React from "react";
import type { Address } from "@left-curve/types";

import { resetAppletsKitMocks, setAppletsKitUseMediaQuery } from "./mocks/applets-kit";
import { createTestQueryClient } from "./utils/query-client";

import { SearchItem } from "../src/components/foundation/SearchItem";

const searchItemMocks = vi.hoisted(() => ({
  accounts: [] as Array<{ address: Address; index: number }>,
  getAccountInfo: vi.fn(),
  getContractInfo: vi.fn(),
  username: undefined as string | undefined,
}));

vi.mock("framer-motion", () => ({
  motion: {
    div: ({
      children,
      variants: _variants,
      ...props
    }: React.HTMLAttributes<HTMLDivElement> & {
      variants?: unknown;
    }) => <div {...props}>{children}</div>,
  },
}));

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    accounts: searchItemMocks.accounts,
    username: searchItemMocks.username,
  }),
  useAppConfig: () => ({
    data: {
      addresses: {},
    },
  }),
  useConfig: () => ({
    chain: {
      blockExplorer: {
        accountPage: `https://explorer.example/account/${"$"}{address}`,
        contractPage: `https://explorer.example/contract/${"$"}{address}`,
      },
    },
  }),
  useFavApplets: () => ({
    addFavApplet: vi.fn(),
    favApplets: [],
    removeFavApplet: vi.fn(),
  }),
  usePublicClient: () => ({
    getAccountInfo: searchItemMocks.getAccountInfo,
    getContractInfo: searchItemMocks.getContractInfo,
  }),
}));

const accountAddress = "0x6163636f756e7400000000000000000000000000";
const contractAddress = "0x636f6e7472616374000000000000000000000000";

function truncated(text: string, end = 20) {
  return `${text.slice(0, 8)}${String.fromCharCode(8230)}${text.slice(text.length - end)}`;
}

function renderSearchItem(component: React.ReactNode) {
  const queryClient = createTestQueryClient();
  return render(component, {
    wrapper: ({ children }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    ),
  });
}

describe("SearchItem", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    searchItemMocks.accounts = [];
    searchItemMocks.getAccountInfo.mockResolvedValue(null);
    searchItemMocks.getContractInfo.mockResolvedValue(null);
    searchItemMocks.username = undefined;
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders backend asset search rows with coin identity and price", () => {
    renderSearchItem(
      <SearchItem.Asset
        decimals={6}
        denom="bridge/usdc"
        logoURI="/images/coins/usd.svg"
        name="USD Coin"
        price="1.00"
        symbol="USDC"
        type="native"
      />,
    );

    expect(screen.getByAltText("USDC")).toHaveAttribute("src", "/images/coins/usd.svg");
    expect(screen.getAllByText("USDC")).toHaveLength(2);
    expect(screen.getByText("$1.00")).toBeInTheDocument();
  });

  it("uses full account addresses on desktop and truncates them on compact screens", () => {
    const { container, rerender } = renderSearchItem(
      <SearchItem.Account
        account={{
          address: accountAddress,
          index: 0,
          owner: 0,
          username: undefined,
        }}
      />,
    );

    expect(screen.getByText("Account #0")).toBeInTheDocument();
    expect(screen.getByText(accountAddress)).toBeInTheDocument();

    setAppletsKitUseMediaQuery({ isMd: false });
    rerender(
      <SearchItem.Account
        account={{
          address: accountAddress,
          index: 0,
          owner: 0,
          username: undefined,
        }}
      />,
    );

    expect(container).not.toHaveTextContent(accountAddress);
    expect(container).toHaveTextContent(truncated(accountAddress));
  });

  it("uses full contract addresses on desktop and truncates them on compact screens", async () => {
    searchItemMocks.getContractInfo.mockResolvedValue({ label: "dex" });

    const { container, rerender } = renderSearchItem(
      <SearchItem.Contract
        contract={{
          address: contractAddress,
          codeHash: "0x636f646500000000000000000000000000000000000000000000000000000000",
          label: "dex",
        }}
      />,
    );

    expect(screen.getByText(contractAddress)).toBeInTheDocument();
    expect(await screen.findByText("dex")).toBeInTheDocument();

    setAppletsKitUseMediaQuery({ isMd: false });
    rerender(
      <SearchItem.Contract
        contract={{
          address: contractAddress,
          codeHash: "0x636f646500000000000000000000000000000000000000000000000000000000",
          label: "dex",
        }}
      />,
    );

    await waitFor(() => {
      expect(container).not.toHaveTextContent(contractAddress);
    });
    expect(container).toHaveTextContent(truncated(contractAddress));
  });

  it("renders backend users with zero accounts as an explicit zero-account result", () => {
    renderSearchItem(
      <SearchItem.User
        user={{
          accounts: {},
          index: 0,
          keys: {},
          name: "genesis",
        }}
      />,
    );

    expect(screen.getByAltText("user")).toHaveAttribute("src", "/images/avatar.png");
    expect(screen.getByText("genesis")).toBeInTheDocument();
    expect(screen.getByText("0 accounts")).toBeInTheDocument();
  });
});
