import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { setAppletsKitUseApp } from "./mocks/applets-kit";

import { MyCommission } from "../src/components/points/referral";

const myCommissionMocks = vi.hoisted(() => ({
  capturedQueries: [] as Array<{
    enabled: boolean;
    queryFn: () => Promise<unknown>;
    queryKey: unknown[];
  }>,
  queryReferralData: vi.fn(),
  useAccount: vi.fn(),
  useAppConfig: vi.fn(),
  usePublicClient: vi.fn(),
  useQueries: vi.fn(),
  useRefereeStats: vi.fn(),
  useReferralData: vi.fn(),
}));

class MockResizeObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

const publicClient = {
  id: "public-client",
};

const perpsAddress = "0x7065727073000000000000000000000000000000";

vi.mock("@left-curve/store", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/store")>();

  return {
    ...actual,
    queryReferralData: myCommissionMocks.queryReferralData,
    useAccount: myCommissionMocks.useAccount,
    useAppConfig: myCommissionMocks.useAppConfig,
    usePublicClient: myCommissionMocks.usePublicClient,
    useRefereeStats: myCommissionMocks.useRefereeStats,
    useReferralData: myCommissionMocks.useReferralData,
  };
});

vi.mock("@tanstack/react-query", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-query")>();

  return {
    ...actual,
    useQueries: myCommissionMocks.useQueries,
  };
});

vi.mock("../src/components/points/referral/StatisticsChart", async () => {
  const React = await import("react");

  return {
    default: ({ metric, period }: { metric: string; period: string }) =>
      React.createElement("div", {
        "data-metric": metric,
        "data-period": period,
        "data-testid": "statistics-chart",
      }),
  };
});

function setAccount({
  isConnected = true,
  userIndex = 42,
}: Partial<{
  isConnected: boolean;
  userIndex: number | undefined;
}> = {}) {
  myCommissionMocks.useAccount.mockReturnValue({
    isConnected,
    userIndex,
  });
}

function setDailyCommissionSnapshots({ isLoading = false }: { isLoading?: boolean } = {}) {
  myCommissionMocks.useQueries.mockImplementation(
    ({
      queries,
    }: {
      queries: Array<{
        enabled: boolean;
        queryFn: () => Promise<unknown>;
        queryKey: unknown[];
      }>;
    }) => {
      myCommissionMocks.capturedQueries = queries;

      return queries.map((_, index) => {
        if (isLoading) {
          return {
            isLoading: true,
          };
        }

        const cumulative = queries.length - index;
        return {
          data: {
            commissionEarnedFromReferees: String(cumulative * 100),
            cumulativeDailyActiveReferees: cumulative,
            refereesVolume: String(cumulative * 1000),
          },
          isLoading: false,
        };
      });
    },
  );
}

function setReferees({
  isLoading = false,
  referees = [
    {
      commissionEarned: "678",
      lastDayActive: 0,
      registeredAt: Date.parse("2026-06-01T00:00:00Z") / 1000,
      userIndex: 7,
      volume: "12345",
    },
    {
      commissionEarned: "25",
      lastDayActive: 0,
      registeredAt: Date.parse("2026-06-02T00:00:00Z") / 1000,
      userIndex: 8,
      volume: "900",
    },
  ],
}: Partial<{
  isLoading: boolean;
  referees: Array<{
    commissionEarned: string;
    lastDayActive: number;
    registeredAt: number;
    userIndex: number;
    volume: string;
  }>;
}> = {}) {
  myCommissionMocks.useRefereeStats.mockReturnValue({
    isLoading,
    referees,
  });
}

function setReferralData({
  isLoading = false,
  referralData = {
    commissionSharedByReferrer: "75",
    volume: "12500",
  },
}: Partial<{
  isLoading: boolean;
  referralData: {
    commissionSharedByReferrer?: string;
    volume?: string;
  };
}> = {}) {
  myCommissionMocks.useReferralData.mockReturnValue({
    isLoading,
    referralData,
  });
}

function bodyRows() {
  return screen.getAllByRole("row").slice(1);
}

function rowText(row: HTMLElement) {
  return row.textContent ?? "";
}

describe("MyCommission", () => {
  beforeEach(() => {
    setAppletsKitUseApp({
      settings: {
        dateFormat: "MMM d, yyyy",
        formatNumberOptions: {
          language: "en-US",
        },
      },
    });
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-08T12:00:00Z"));

    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: MockResizeObserver,
    });

    myCommissionMocks.queryReferralData.mockResolvedValue({});
    myCommissionMocks.usePublicClient.mockReturnValue(publicClient);
    myCommissionMocks.useAppConfig.mockReturnValue({
      data: {
        addresses: {
          perps: perpsAddress,
        },
      },
    });
    setAccount();
    setDailyCommissionSnapshots();
    setReferees();
    setReferralData();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
    myCommissionMocks.capturedQueries = [];
  });

  it("derives daily affiliate commission rows from cumulative referral query snapshots", async () => {
    render(<MyCommission mode="affiliate" />);

    expect(screen.getByRole("button", { name: m["referral.commission.myCommission"]() }));
    expect(myCommissionMocks.useQueries).toHaveBeenCalledOnce();
    expect(myCommissionMocks.capturedQueries).toHaveLength(11);
    expect(myCommissionMocks.capturedQueries[0].queryKey[0]).toBe("referralData");
    expect(myCommissionMocks.capturedQueries[0].queryKey[1]).toBe(42);
    expect(myCommissionMocks.capturedQueries.every((query) => query.enabled)).toBe(true);

    await myCommissionMocks.capturedQueries[0].queryFn();

    expect(myCommissionMocks.queryReferralData).toHaveBeenCalledWith(
      publicClient,
      perpsAddress,
      42,
      expect.any(Number),
    );

    const rows = bodyRows();
    expect(rows).toHaveLength(10);
    expect(rowText(rows[0])).toMatch(/\$100(?:\.00)?/);
    expect(rowText(rows[0])).toMatch(/\$1,000(?:\.00)?/);
    expect(rowText(rows[0])).toContain("1");
    expect(rowText(rows[0])).toContain("Jun 8, 2026");
  });

  it("renders partial backend commission snapshots as zero-valued affiliate rows", () => {
    myCommissionMocks.useQueries.mockImplementation(
      ({
        queries,
      }: {
        queries: Array<{
          enabled: boolean;
          queryFn: () => Promise<unknown>;
          queryKey: unknown[];
        }>;
      }) => {
        myCommissionMocks.capturedQueries = queries;
        return queries.map(() => ({
          data: {},
          isLoading: false,
        }));
      },
    );

    render(<MyCommission mode="affiliate" />);

    const rows = bodyRows();
    expect(rows).toHaveLength(10);
    expect(rowText(rows[0])).toMatch(/\$0(?:\.00)?/);
    expect(rowText(rows[0])).toContain("0");
    expect(rowText(rows[0])).toContain("Jun 8, 2026");
    expect(screen.queryByText(m["referral.commission.noReferees"]())).not.toBeInTheDocument();
  });

  it("requests a new daily affiliate commission window when changing pages", async () => {
    render(<MyCommission mode="affiliate" />);

    const firstPageQueryKeys = myCommissionMocks.capturedQueries.map((query) => query.queryKey);
    expect(firstPageQueryKeys).toHaveLength(11);

    fireEvent.click(screen.getByRole("button", { name: "2" }));

    expect(myCommissionMocks.useQueries).toHaveBeenCalledTimes(2);

    const secondPageQueries = myCommissionMocks.capturedQueries;
    const secondPageQueryKeys = secondPageQueries.map((query) => query.queryKey);
    expect(secondPageQueryKeys).toHaveLength(11);
    expect(secondPageQueryKeys[0]).toEqual(["referralData", 42, expect.any(Number)]);
    expect(secondPageQueryKeys[0][2]).toBeLessThan(firstPageQueryKeys[0][2] as number);
    expect(secondPageQueryKeys.at(-1)?.[2]).toBe(firstPageQueryKeys[0][2]);

    await secondPageQueries[0].queryFn();

    expect(myCommissionMocks.queryReferralData).toHaveBeenCalledWith(
      publicClient,
      perpsAddress,
      42,
      secondPageQueryKeys[0][2],
    );
  });

  it("renders affiliate referee rows from backend referee stats", () => {
    render(<MyCommission mode="affiliate" />);

    fireEvent.click(screen.getByRole("button", { name: m["referral.commission.myReferees"]() }));

    expect(myCommissionMocks.useRefereeStats).toHaveBeenCalledWith({
      referrerIndex: 42,
    });

    const rows = bodyRows();
    expect(rows).toHaveLength(2);
    expect(rowText(rows[0])).toContain("#7");
    expect(rowText(rows[0])).toMatch(/\$12,345(?:\.00)?/);
    expect(rowText(rows[0])).toMatch(/\$678(?:\.00)?/);
    expect(rowText(rows[0])).toContain("Jun 1, 2026");
    expect(rowText(rows[1])).toContain("#8");
  });

  it("renders backend referee index zero and zero totals as real referee rows", () => {
    setReferees({
      referees: [
        {
          commissionEarned: "0",
          lastDayActive: 0,
          registeredAt: 0,
          userIndex: 0,
          volume: "0",
        },
      ],
    });

    render(<MyCommission mode="affiliate" />);

    fireEvent.click(screen.getByRole("button", { name: m["referral.commission.myReferees"]() }));

    const rows = bodyRows();
    expect(rows).toHaveLength(1);
    expect(rowText(rows[0])).toContain("#0");
    expect(rowText(rows[0]).match(/\$0(?:\.00)?/g)).toHaveLength(2);
    expect(rowText(rows[0])).toContain("Jan 1, 1970");
  });

  it("routes affiliate statistics tab state into the backend-powered chart", async () => {
    vi.useRealTimers();

    render(<MyCommission mode="affiliate" />);

    fireEvent.click(screen.getByRole("button", { name: m["referral.commission.statistics"]() }));

    const chart = await screen.findByTestId("statistics-chart");
    expect(chart).toHaveAttribute("data-metric", "commission");
    expect(chart).toHaveAttribute("data-period", "7D");
  });

  it("renders trader rebate rows from referral data", () => {
    render(<MyCommission mode="trader" />);

    expect(screen.getByRole("button", { name: m["referral.rebate.myRebates"]() }));
    expect(myCommissionMocks.useReferralData).toHaveBeenCalledWith({
      userIndex: 42,
    });

    const rows = bodyRows();
    expect(rows).toHaveLength(1);
    expect(rowText(rows[0])).toMatch(/\$75(?:\.00)?/);
    expect(rowText(rows[0])).toMatch(/\$12,500(?:\.00)?/);
    expect(rowText(rows[0])).toContain("Jun 8, 2026");
  });

  it("preserves backend zero rebates in trader rows when trading volume is positive", () => {
    setReferralData({
      referralData: {
        commissionSharedByReferrer: "0",
        volume: "5000",
      },
    });

    render(<MyCommission mode="trader" />);

    const rows = bodyRows();
    expect(rows).toHaveLength(1);
    expect(rowText(rows[0])).toMatch(/\$0(?:\.00)?/);
    expect(rowText(rows[0])).toMatch(/\$5,000(?:\.00)?/);
    expect(rowText(rows[0])).toContain("Jun 8, 2026");
    expect(screen.queryByText(m["referral.commission.noRebates"]())).not.toBeInTheDocument();
  });

  it("preserves backend zero trading volume in trader rows when rebates are positive", () => {
    setReferralData({
      referralData: {
        commissionSharedByReferrer: "25",
        volume: "0",
      },
    });

    render(<MyCommission mode="trader" />);

    const rows = bodyRows();
    expect(rows).toHaveLength(1);
    expect(rowText(rows[0])).toMatch(/\$25(?:\.00)?/);
    expect(rowText(rows[0])).toMatch(/\$0(?:\.00)?/);
    expect(rowText(rows[0])).toContain("Jun 8, 2026");
    expect(screen.queryByText(m["referral.commission.noRebates"]())).not.toBeInTheDocument();
  });

  it("hides the section for disconnected users and shows empty rebate state when connected data is zero", () => {
    setAccount({
      isConnected: false,
      userIndex: undefined,
    });

    const { container, rerender } = render(<MyCommission mode="trader" />);

    expect(container).toBeEmptyDOMElement();

    setAccount();
    setReferralData({
      referralData: {
        commissionSharedByReferrer: "0",
        volume: "0",
      },
    });

    rerender(<MyCommission mode="trader" />);

    expect(screen.getByText(m["referral.commission.noRebates"]())).toBeInTheDocument();
  });
});
