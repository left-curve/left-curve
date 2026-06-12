import { vi } from "vitest";

import type React from "react";

type SubmitTxMutation = {
  mutationFn: (
    variables: unknown,
    options: { abort: () => void; signal: AbortSignal },
  ) => Promise<unknown>;
  onSuccess?: (result: unknown) => void;
};

class TestResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

vi.stubGlobal("ResizeObserver", TestResizeObserver);

const transferMocks = vi.hoisted(() => ({
  balances: {
    "bridge/usdc": "100000000",
  } as Record<string, string>,
  changeAction: vi.fn(),
  coinsByDenom: {
    "bridge/usdc": {
      decimals: 6,
      denom: "bridge/usdc",
      logoURI: "/usdc.png",
      symbol: "USDC",
    },
  } as Record<string, { decimals: number; denom: string; logoURI: string; symbol: string }>,
  depositMargin: vi.fn(),
  getAccountInfo: vi.fn(),
  hasSigningClient: true,
  invalidatePerpsAccountResources: vi.fn(),
  isConnected: true,
  isValidAddress: vi.fn(),
  queryInvalidate: vi.fn(),
  refreshBalances: vi.fn(),
  showModal: vi.fn(),
  transfer: vi.fn(),
  useQuery: vi.fn(),
  withdrawMargin: vi.fn(),
}));

export function getTransferMocks() {
  return transferMocks;
}

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, to, ...props }: React.PropsWithChildren<{ to?: string }>) => (
    <a href={to} {...props}>
      {children}
    </a>
  ),
}));

vi.mock("@tanstack/react-query", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-query")>();

  return {
    ...actual,
    useQuery: transferMocks.useQuery,
    useQueryClient: () => ({
      invalidateQueries: transferMocks.queryInvalidate,
    }),
  };
});

vi.mock("@left-curve/sdk", async (importOriginal) => {
  const actual = await importOriginal<object>();

  return {
    ...actual,
    isValidAddress: transferMocks.isValidAddress,
  };
});

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
      showModal: transferMocks.showModal,
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  invalidatePerpsAccountResources: transferMocks.invalidatePerpsAccountResources,
  useAccount: () => ({
    account: transferMocks.isConnected
      ? { address: "0x73656e6465720000000000000000000000000000" }
      : undefined,
    isConnected: transferMocks.isConnected,
    username: "alice",
  }),
  useAppConfig: () => ({
    data: {
      addresses: {
        perps: "0x7065727073000000000000000000000000000000",
      },
    },
  }),
  useBalances: () => ({
    data: transferMocks.balances,
    refetch: transferMocks.refreshBalances,
  }),
  useConfig: () => ({
    chain: {
      id: "dango-dev-1",
    },
    coins: {
      byDenom: transferMocks.coinsByDenom,
      getCoinInfo: (denom: string) => transferMocks.coinsByDenom[denom],
    },
  }),
  usePerpsUserStateExtended: () => "3.5",
  usePrices: () => ({
    getPrice: () => "0",
  }),
  usePublicClient: () => ({
    getAccountInfo: transferMocks.getAccountInfo,
  }),
  useSigningClient: () => ({
    data: transferMocks.hasSigningClient
      ? {
          depositMargin: transferMocks.depositMargin,
          transfer: transferMocks.transfer,
          withdrawMargin: transferMocks.withdrawMargin,
        }
      : undefined,
  }),
  useSubmitTx: ({ mutation }: { mutation: SubmitTxMutation }) => ({
    isPending: false,
    mutateAsync: async (variables: unknown) => {
      try {
        const result = await mutation.mutationFn(variables, {
          abort: () => {
            throw new Error("aborted");
          },
          signal: new AbortController().signal,
        });
        mutation.onSuccess?.(result);
        return result;
      } catch {
        return undefined;
      }
    },
  }),
}));

vi.mock("@left-curve/applets-kit", async (importOriginal) => {
  const React = await import("react");

  const actual = await importOriginal<typeof import("@left-curve/applets-kit")>();

  return {
    ...actual,
    AccountSearchInput: ({
      errorMessage: _errorMessage,
      label,
      isDisabled,
      ...props
    }: React.ComponentProps<"input"> & {
      errorMessage?: string;
      label: string;
      isDisabled?: boolean;
    }) => (
      <label>
        {label}
        <input aria-label={label} disabled={isDisabled} {...props} />
      </label>
    ),
    CoinSelector: ({
      coins,
      isDisabled,
      onChange,
      value,
    }: {
      coins: Array<{ denom: string; symbol: string }>;
      isDisabled?: boolean;
      onChange: (denom: string) => void;
      value?: string;
    }) => (
      <select
        aria-label="coin selector"
        data-testid="coin-selector"
        disabled={isDisabled}
        onChange={(event) => onChange(event.currentTarget.value)}
        value={value}
      >
        {coins.map((coin) => (
          <option key={coin.denom} value={coin.denom}>
            {coin.symbol}
          </option>
        ))}
      </select>
    ),
    Tab: ({ children, title }: React.PropsWithChildren<{ title: string }>) => (
      <span>{children ?? title}</span>
    ),
    Tabs: ({
      children,
      isDisabled,
      keys,
      onTabChange,
      selectedTab,
    }: React.PropsWithChildren<{
      isDisabled?: boolean;
      keys?: string[];
      onTabChange?: (tab: string) => void;
      selectedTab?: string;
    }>) => {
      const childTabs = React.Children.toArray(children).filter(
        (child): child is React.ReactElement<{ title: string }> =>
          React.isValidElement<{ title: string }>(child),
      );
      const tabs = keys
        ? keys.map((title) => ({ title, label: title }))
        : childTabs.map((child) => ({ title: child.props.title, label: child }));

      return (
        <div>
          {tabs.map(({ title, label }) => (
            <button
              aria-pressed={title === selectedTab}
              disabled={isDisabled}
              key={title}
              onClick={() => onTabChange?.(title)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
      );
    },
    useApp: () => ({
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
      showModal: transferMocks.showModal,
    }),
  };
});
