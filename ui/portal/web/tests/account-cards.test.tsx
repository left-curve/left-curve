import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { Modals } from "@left-curve/applets-kit";

import type React from "react";

import { AccountCard } from "../src/components/foundation/AccountCard";
import { ContractCard } from "../src/components/foundation/ContractCard";
import { createTestQueryClient } from "./utils/query-client";

const accountCardMocks = vi.hoisted(() => ({
  calculateBalance: vi.fn(),
  getAccountInfo: vi.fn(),
  getContractInfo: vi.fn(),
  showModal: vi.fn(),
  useBalances: vi.fn(),
}));

type AccountFixture = {
  address: string;
  index: number;
};

vi.mock("framer-motion", () => ({
  AnimatePresence: ({ children }: React.PropsWithChildren) => <>{children}</>,
  motion: {
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

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    accounts: [],
    username: undefined,
  }),
  useAppConfig: () => ({
    data: {
      addresses: {},
    },
  }),
  useBalances: accountCardMocks.useBalances,
  useConfig: () => ({
    chain: {
      blockExplorer: {
        accountPage: `https://explorer.example/account/${"$"}{address}`,
        contractPage: `https://explorer.example/contract/${"$"}{address}`,
      },
    },
  }),
  usePrices: () => ({
    calculateBalance: accountCardMocks.calculateBalance,
  }),
  usePublicClient: () => ({
    getAccountInfo: accountCardMocks.getAccountInfo,
    getContractInfo: accountCardMocks.getContractInfo,
  }),
}));

const account: AccountFixture = {
  address: "0x6163636f756e7400000000000000000000000000",
  index: 7,
};

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();

  return render(<QueryClientProvider client={queryClient}>{component}</QueryClientProvider>);
}

function expectTruncatedText(start: string, end: string) {
  const truncatedText = screen.getByText(start).parentElement;

  expect(truncatedText).toHaveTextContent(`${start}…${end}`);
}

function expectResponsiveText(start: string, end: string) {
  const responsiveText = screen.getByText(start).parentElement;

  expect(responsiveText).toHaveTextContent(`${start}${end}`);
}

function pointerUpCopyButtonFor(start: string) {
  const addressRow = screen.getByText(start).closest("div");

  expect(addressRow).not.toBeNull();

  const copyButton = within(addressRow as HTMLElement).getByRole("button");
  fireEvent.pointerUp(copyButton);

  return copyButton;
}

describe("foundation account and contract cards", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      settings: {
        formatNumberOptions: {
          currency: "USD",
          language: "en-US",
        },
      },
      showModal: accountCardMocks.showModal,
    });
    accountCardMocks.calculateBalance.mockReturnValue("$123.45");
    accountCardMocks.getAccountInfo.mockResolvedValue(null);
    accountCardMocks.getContractInfo.mockResolvedValue(null);
    accountCardMocks.useBalances.mockReturnValue({
      data: {
        "bridge/usdc": "123450000",
      },
    });
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: vi.fn(),
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders an account card with active status, balances, selector action, and copy warning", () => {
    const onTriggerAction = vi.fn();

    const { container } = renderWithQueryClient(
      <AccountCard
        account={account}
        balance="$120.00"
        balanceChange="+2.4%"
        isSelectorActive={false}
        isUserActive
        onTriggerAction={onTriggerAction}
      />,
    );

    expect(screen.getByText(`${m["common.account"]()} #7`)).toBeInTheDocument();
    expect(screen.getByText("Active")).toHaveClass("bg-surface-secondary-blue");
    expectTruncatedText("0x61", "0000");
    expect(screen.getByText("$120.00")).toBeInTheDocument();
    expect(screen.getByText("+2.4%")).toBeInTheDocument();

    const actionButton = container.querySelector("button.absolute");

    expect(actionButton).not.toBeNull();

    fireEvent.click(actionButton as HTMLButtonElement);
    expect(onTriggerAction).toHaveBeenCalledOnce();

    pointerUpCopyButtonFor("0x61");

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(account.address);
    expect(accountCardMocks.showModal).toHaveBeenCalledWith(Modals.AddressWarning);
  });

  it("renders an inactive selected account card with the close selector icon", () => {
    const { container } = renderWithQueryClient(
      <AccountCard
        account={account}
        balance="$0.00"
        isSelectorActive
        isUserActive={false}
        onTriggerAction={vi.fn()}
      />,
    );

    expect(screen.getByText(m["explorer.user.inactive"]())).toHaveClass("bg-utility-gray-100");
    expect(container.querySelector("button.absolute svg.w-5.h-5")).not.toBeNull();
  });

  it("selects preview accounts and calculates their formatted aggregate balance", () => {
    const onAccountSelect = vi.fn();

    renderWithQueryClient(
      <AccountCard.Preview account={account} onAccountSelect={onAccountSelect} />,
    );

    expect(accountCardMocks.useBalances).toHaveBeenCalledWith({ address: account.address });
    expect(accountCardMocks.calculateBalance).toHaveBeenCalledWith(
      {
        "bridge/usdc": "123450000",
      },
      {
        format: true,
        formatOptions: {
          currency: "usd",
          language: "en-US",
        },
      },
    );
    expect(screen.getByText("$123.45")).toBeInTheDocument();
    expectTruncatedText("0x61", "0000");

    pointerUpCopyButtonFor("0x61");

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(account.address);
    expect(accountCardMocks.showModal).toHaveBeenCalledWith(Modals.AddressWarning);

    fireEvent.click(screen.getByText(`${m["common.account"]()} #7`));
    expect(onAccountSelect).toHaveBeenCalledWith(account);
  });

  it("renders contract cards with visualized address, app badge, copy target, and balance", () => {
    const contractAddress = "0x636f6e7472616374000000000000000000000000";

    renderWithQueryClient(<ContractCard address={contractAddress} balance="$999.00" />);

    expect(screen.getByText("App")).toHaveClass("bg-surface-tertiary-green");
    expectResponsiveText("0x636f6e7472616374000000000000000000", "000000");
    expectTruncatedText("0x63", "0000");

    pointerUpCopyButtonFor("0x63");

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(contractAddress);
    expect(screen.getByText("$999.00")).toBeInTheDocument();
  });
});
