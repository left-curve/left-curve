import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { Modals } from "@left-curve/applets-kit";
import { UserWithdrawals } from "../src/components/earn/UserWithdrawals";
import { VaultPerformanceChart } from "../src/components/earn/VaultPerformanceChart";
import { VaultLiquidity } from "../src/components/earn/VaultLiquidity";

const vaultLiquidityMocks = vi.hoisted(() => ({
  depositMutateAsync: vi.fn(),
  showModal: vi.fn(),
  useAccount: vi.fn(),
  useVaultLiquidityState: vi.fn(),
  useVaultSnapshots: vi.fn(),
  withdrawMutateAsync: vi.fn(),
}));

type LiquidityStateOverrides = Partial<{
  isLoading: boolean;
  isPaused: boolean;
  isTvlCapReached: boolean;
  sharesToReceive: string;
  userHasShares: boolean;
  userMargin: string;
  userSharesValue: string;
  userUnlocks: unknown[];
  userVaultShares: string;
  usdToReceive: string;
  vaultApy: string | null;
  vaultState: { equity: string };
}>;

let liquidityStateOverrides: LiquidityStateOverrides = {};

class TestResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

vi.stubGlobal("ResizeObserver", TestResizeObserver);

vi.mock("@tanstack/react-router", () => ({
  useRouter: () => ({
    history: {
      go: vi.fn(),
    },
  }),
}));

vi.mock("recharts", () => ({
  Bar: ({ children }: React.PropsWithChildren) => <div data-testid="chart-bar">{children}</div>,
  CartesianGrid: () => <div data-testid="chart-grid" />,
  Cell: ({ fill }: { fill: string }) => <span data-fill={fill} data-testid="chart-cell" />,
  ComposedChart: ({ children, data }: React.PropsWithChildren<{ data: Array<unknown> }>) => (
    <div data-points={data.length} data-testid="composed-chart">
      {children}
    </div>
  ),
  Line: () => <div data-testid="chart-line" />,
  ReferenceLine: () => <div data-testid="reference-line" />,
  ResponsiveContainer: ({ children }: React.PropsWithChildren) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  Tooltip: () => <div data-testid="chart-tooltip" />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
}));

vi.mock("@left-curve/foundation", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@left-curve/foundation")>()),
  useApp: () => ({
    settings: {
      formatNumberOptions: {
        currency: "USD",
        language: "en-US",
        mask: 1,
      },
    },
  }),
}));

vi.mock("@left-curve/store", () => ({
  perpsMarginAsset: {
    decimals: 6,
    denom: "bridge/usdc",
    logoURI: "/images/coins/usd.svg",
    name: "USD Coin",
    symbol: "USDC",
  },
  useAccount: vaultLiquidityMocks.useAccount,
  useVaultLiquidityState: vaultLiquidityMocks.useVaultLiquidityState,
  useVaultSnapshots: vaultLiquidityMocks.useVaultSnapshots,
}));

const account = {
  address: "0x7661756c746c6971756964697479000000000000",
};

function defaultLiquidityState(action: "deposit" | "withdraw", onChangeAction: unknown) {
  return {
    action,
    deposit: {
      isPending: false,
      mutateAsync: vaultLiquidityMocks.depositMutateAsync,
    },
    isLoading: false,
    isPaused: false,
    isTvlCapReached: false,
    onChangeAction,
    sharesToReceive: "12.5",
    userHasShares: true,
    userMargin: "1000",
    userSharesValue: "500",
    userUnlocks: [],
    userVaultShares: "333",
    usdToReceive: "222",
    vaultApy: "8.75",
    vaultState: {
      equity: "2500000",
    },
    withdraw: {
      isPending: false,
      isSuccess: false,
      mutateAsync: vaultLiquidityMocks.withdrawMutateAsync,
    },
    ...liquidityStateOverrides,
  };
}

function renderVaultLiquidity({
  initialAction = "deposit",
  onChangeAction = vi.fn(),
}: {
  initialAction?: "deposit" | "withdraw";
  onChangeAction?: (action: "deposit" | "withdraw") => void;
} = {}) {
  function Harness() {
    const [action, setAction] = useState<"deposit" | "withdraw">(initialAction);

    return (
      <VaultLiquidity
        action={action}
        onChangeAction={(nextAction) => {
          onChangeAction(nextAction);
          setAction(nextAction);
        }}
      >
        <VaultLiquidity.Content />
      </VaultLiquidity>
    );
  }

  return render(<Harness />);
}

function inputByName(name: string) {
  const input = document.querySelector<HTMLInputElement>(`input[name="${name}"]`);
  if (!input) throw new Error(`Expected input named ${name}`);
  return input;
}

function changePerformancePeriod(currentPeriod: string, nextPeriod: string) {
  fireEvent.click(
    screen.getByRole("button", {
      name: (name) => name.includes(currentPeriod),
    }),
  );

  const option = screen.getAllByText(nextPeriod).at(-1);
  if (!option) throw new Error(`Expected performance period option ${nextPeriod}`);
  fireEvent.click(option);
}

describe("vault liquidity screen", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      settings: {
        formatNumberOptions: {
          currency: "USD",
          language: "en-US",
          mask: 1,
        },
      },
      showModal: vaultLiquidityMocks.showModal,
    });
    liquidityStateOverrides = {};
    vaultLiquidityMocks.useAccount.mockReturnValue({
      account,
    });
    vaultLiquidityMocks.useVaultLiquidityState.mockImplementation(
      ({
        action,
        onChangeAction,
      }: {
        action: "deposit" | "withdraw";
        onChangeAction: (action: "deposit" | "withdraw") => void;
      }) => defaultLiquidityState(action, onChangeAction),
    );
    vaultLiquidityMocks.useVaultSnapshots.mockReturnValue({
      data: [
        {
          dailyChange: 0.7,
          date: "2026-06-01T12:00:00.000Z",
          sharePrice: 1.25,
        },
        {
          dailyChange: -0.2,
          date: "2026-06-02T12:00:00.000Z",
          sharePrice: 1.31,
        },
      ],
      error: null,
      isLoading: false,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("maps the selected performance period into the vault state and snapshot queries", async () => {
    renderVaultLiquidity();

    expect(vaultLiquidityMocks.useVaultLiquidityState).toHaveBeenCalledWith(
      expect.objectContaining({
        action: "deposit",
        apyWindowDays: 14,
      }),
    );
    expect(vaultLiquidityMocks.useVaultSnapshots).toHaveBeenCalledWith({
      period: "14D",
    });
    expect(screen.getByTestId("composed-chart")).toHaveAttribute("data-points", "2");

    changePerformancePeriod("14D", "30D");

    await waitFor(() =>
      expect(vaultLiquidityMocks.useVaultLiquidityState).toHaveBeenLastCalledWith(
        expect.objectContaining({
          apyWindowDays: 30,
        }),
      ),
    );
    expect(vaultLiquidityMocks.useVaultSnapshots).toHaveBeenLastCalledWith({
      period: "30D",
    });
  });

  it("renders backend zero APY and TVL as real vault metrics", () => {
    liquidityStateOverrides = {
      sharesToReceive: "0",
      userMargin: "0",
      userSharesValue: "0",
      vaultApy: "0",
      vaultState: {
        equity: "0",
      },
    };

    renderVaultLiquidity();

    expect(screen.getByText("APY (14D)")).toBeInTheDocument();
    expect(screen.getAllByText("0%").length).toBeGreaterThanOrEqual(1);
    expect(screen.queryByText("-")).not.toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.tvl"]())).toBeInTheDocument();
    expect(screen.getAllByText("$0.00").length).toBeGreaterThanOrEqual(1);
  });

  it("moves a no-share user back to deposit and hides the withdraw tab", async () => {
    const onChangeAction = vi.fn();
    liquidityStateOverrides = {
      userHasShares: false,
      userSharesValue: "0",
      userVaultShares: "0",
    };

    renderVaultLiquidity({
      initialAction: "withdraw",
      onChangeAction,
    });

    await waitFor(() => expect(onChangeAction).toHaveBeenCalledWith("deposit"));
    expect(screen.getAllByRole("button", { name: "deposit" })).toHaveLength(2);
    expect(screen.queryAllByRole("button", { name: "withdraw" })).toHaveLength(0);
    for (const button of screen.getAllByRole("button", { name: m["common.deposit"]() })) {
      expect(button).toBeDisabled();
    }
  });

  it("opens the add-liquidity confirmation with the entered amount and share estimate", () => {
    renderVaultLiquidity();

    for (const button of screen.getAllByRole("button", { name: m["common.deposit"]() })) {
      expect(button).toBeDisabled();
    }

    fireEvent.change(inputByName("depositAmount"), {
      target: {
        value: "125",
      },
    });
    const depositButton = screen.getAllByRole("button", { name: m["common.deposit"]() })[0];
    expect(depositButton).not.toBeDisabled();

    fireEvent.click(depositButton);

    expect(vaultLiquidityMocks.showModal).toHaveBeenCalledWith("vault-add-liquidity", {
      amount: "125",
      confirmAddLiquidity: vaultLiquidityMocks.depositMutateAsync,
      sharesToReceive: "12.5",
    });
  });

  it("routes logged-out users to authentication instead of liquidity transaction modals", () => {
    vaultLiquidityMocks.useAccount.mockReturnValue({
      account: undefined,
    });

    renderVaultLiquidity();

    fireEvent.click(screen.getAllByRole("button", { name: m["common.signin"]() })[0]);

    expect(vaultLiquidityMocks.showModal).toHaveBeenCalledWith(Modals.Authenticate);
    expect(vaultLiquidityMocks.showModal).not.toHaveBeenCalledWith(
      Modals.VaultAddLiquidity,
      expect.anything(),
    );

    cleanup();
    vi.clearAllMocks();
    vaultLiquidityMocks.useAccount.mockReturnValue({
      account: undefined,
    });

    renderVaultLiquidity({
      initialAction: "withdraw",
    });

    fireEvent.click(screen.getAllByRole("button", { name: m["common.signin"]() })[0]);

    expect(vaultLiquidityMocks.showModal).toHaveBeenCalledWith(Modals.Authenticate);
    expect(vaultLiquidityMocks.showModal).not.toHaveBeenCalledWith(
      Modals.VaultWithdrawLiquidity,
      expect.anything(),
    );
  });

  it("honors paused and full vault backend states before opening deposit modals", () => {
    liquidityStateOverrides = {
      isPaused: true,
    };

    renderVaultLiquidity();

    expect(screen.getByText(m["vaultLiquidity.paused"]())).toBeInTheDocument();

    fireEvent.change(inputByName("depositAmount"), {
      target: {
        value: "125",
      },
    });

    for (const button of screen.getAllByRole("button", { name: m["common.deposit"]() })) {
      expect(button).toBeDisabled();
    }
    expect(vaultLiquidityMocks.showModal).not.toHaveBeenCalled();

    cleanup();
    vi.clearAllMocks();
    liquidityStateOverrides = {
      isTvlCapReached: true,
    };

    renderVaultLiquidity();

    expect(screen.getAllByText(m["vaultLiquidity.tvlCapReached"]())).toHaveLength(2);

    fireEvent.change(inputByName("depositAmount"), {
      target: {
        value: "125",
      },
    });

    for (const button of screen.getAllByRole("button", { name: m["common.deposit"]() })) {
      expect(button).toBeDisabled();
    }
    expect(vaultLiquidityMocks.showModal).not.toHaveBeenCalled();
  });

  it("honors paused vault backend state before opening withdrawal modals", () => {
    liquidityStateOverrides = {
      isPaused: true,
    };

    renderVaultLiquidity({
      initialAction: "withdraw",
    });

    fireEvent.click(screen.getAllByRole("button", { name: m["common.max"]() })[0]);

    for (const button of screen.getAllByRole("button", { name: m["common.withdraw"]() })) {
      expect(button).toBeDisabled();
    }
    expect(vaultLiquidityMocks.showModal).not.toHaveBeenCalled();
  });

  it("uses the exact share balance for full withdrawals", () => {
    renderVaultLiquidity({
      initialAction: "withdraw",
    });

    const withdrawButton = screen.getAllByRole("button", { name: m["common.withdraw"]() })[0];
    expect(withdrawButton).toBeDisabled();

    fireEvent.click(screen.getAllByRole("button", { name: m["common.max"]() })[0]);
    expect(withdrawButton).not.toBeDisabled();

    fireEvent.click(withdrawButton);

    expect(vaultLiquidityMocks.showModal).toHaveBeenCalledWith("vault-withdraw-liquidity", {
      confirmWithdrawal: vaultLiquidityMocks.withdrawMutateAsync,
      sharesToBurn: "333",
      usdToReceive: "222",
    });
  });

  it("renders vault snapshot loading, error, and empty states", () => {
    const onPeriodChange = vi.fn();
    vaultLiquidityMocks.useVaultSnapshots.mockReturnValueOnce({
      data: undefined,
      error: null,
      isLoading: true,
    });

    const { container, rerender } = render(
      <VaultPerformanceChart period="7D" onPeriodChange={onPeriodChange} />,
    );

    expect(container.querySelector(".animate-spinner-ease-spin")).toBeInTheDocument();
    expect(vaultLiquidityMocks.useVaultSnapshots).toHaveBeenLastCalledWith({
      period: "7D",
    });

    vaultLiquidityMocks.useVaultSnapshots.mockReturnValueOnce({
      data: undefined,
      error: new Error("Snapshot service unavailable"),
      isLoading: false,
    });

    rerender(<VaultPerformanceChart period="14D" onPeriodChange={onPeriodChange} />);

    expect(screen.getByText("Snapshot service unavailable")).toBeInTheDocument();
    expect(vaultLiquidityMocks.useVaultSnapshots).toHaveBeenLastCalledWith({
      period: "14D",
    });

    vaultLiquidityMocks.useVaultSnapshots.mockReturnValueOnce({
      data: [],
      error: null,
      isLoading: false,
    });

    rerender(<VaultPerformanceChart period="30D" onPeriodChange={onPeriodChange} />);

    expect(screen.getByText(m["vaultLiquidity.noData"]())).toBeInTheDocument();
    expect(vaultLiquidityMocks.useVaultSnapshots).toHaveBeenLastCalledWith({
      period: "30D",
    });
  });

  it("emits period changes from the standalone performance chart select", () => {
    const onPeriodChange = vi.fn();

    render(<VaultPerformanceChart period="14D" onPeriodChange={onPeriodChange} />);

    changePerformancePeriod("14D", "90D");

    expect(onPeriodChange).toHaveBeenCalledWith("90D");
  });

  it("renders withdrawal empty and cooldown table states", () => {
    const { rerender } = render(<UserWithdrawals unlocks={[]} />);

    expect(screen.getByText(m["vaultLiquidity.noWithdrawals"]())).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.noWithdrawalsDescription"]())).toBeInTheDocument();

    const unlock = {
      amountToRelease: "1234.56",
      endTime: "1717243200",
    } as React.ComponentProps<typeof UserWithdrawals>["unlocks"][number];

    rerender(<UserWithdrawals unlocks={[unlock]} />);

    expect(screen.getByText(m["vaultLiquidity.withdrawalsDescription"]())).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.usdAmount"]())).toBeInTheDocument();
    expect(screen.getByText(m["vaultLiquidity.cooldownEndTime"]())).toBeInTheDocument();
    expect(screen.getByText("$1,234.56")).toBeInTheDocument();
    expect(screen.getByText("06/01/2024")).toBeInTheDocument();
  });
});
