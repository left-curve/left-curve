import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { LeaderboardTable } from "../src/components/points/leaderboard/LeaderboardTable";

const leaderboardMocks = vi.hoisted(() => ({
  handleSortChange: vi.fn(),
  handleTimeframeChange: vi.fn(),
  setPage: vi.fn(),
  useAccount: vi.fn(),
  useLeaderboard: vi.fn(),
  useUserPoints: vi.fn(),
}));

class MockResizeObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      settings: {
        formatNumberOptions: {},
      },
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: leaderboardMocks.useAccount,
  useLeaderboard: leaderboardMocks.useLeaderboard,
}));

vi.mock("../src/components/points/useUserPoints", () => ({
  useUserPoints: leaderboardMocks.useUserPoints,
}));

type LeaderboardStats = {
  points?: {
    perps?: string;
    referral?: string;
    vault?: string;
  };
  realized_pnl?: string;
  volume?: string;
};

function leaderboardEntry({
  originalRank,
  stats = {},
  userIndex,
  username,
}: {
  originalRank: number;
  stats?: LeaderboardStats;
  userIndex: number;
  username: string | null;
}) {
  return {
    originalRank,
    stats: {
      points: {
        perps: stats.points?.perps ?? "0",
        referral: stats.points?.referral ?? "0",
        vault: stats.points?.vault ?? "0",
      },
      realized_pnl: stats.realized_pnl ?? "0",
      volume: stats.volume ?? "0",
    },
    user_index: userIndex,
    username,
  };
}

function setLeaderboardState({
  entries = [],
  isLoading = false,
  page = 1,
  pinnedUser = null,
  sort = "points",
  sortDirection = "desc",
  timeframe = null,
  totalPages = 1,
}: Partial<{
  entries: unknown[];
  isLoading: boolean;
  page: number;
  pinnedUser: unknown;
  sort: "pnl" | "points" | "volume";
  sortDirection: "asc" | "desc";
  timeframe: null | 1 | 2 | 4;
  totalPages: number;
}> = {}) {
  leaderboardMocks.useLeaderboard.mockReturnValue({
    entries,
    handleSortChange: leaderboardMocks.handleSortChange,
    handleTimeframeChange: leaderboardMocks.handleTimeframeChange,
    isLoading,
    page,
    pinnedUser,
    setPage: leaderboardMocks.setPage,
    sort,
    sortDirection,
    timeframe,
    totalPages,
  });
}

function bodyRows() {
  return screen.getAllByRole("row").slice(1);
}

function rowText(row: HTMLElement) {
  return row.textContent ?? "";
}

describe("LeaderboardTable", () => {
  beforeEach(() => {
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          pointsUrl: "https://points.example",
        },
      },
    });
    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: MockResizeObserver,
    });

    leaderboardMocks.useAccount.mockReturnValue({
      userIndex: 7,
      username: "trader-seven",
    });
    leaderboardMocks.useUserPoints.mockReturnValue({
      pnl: 12,
      points: 321,
      volume: 45000,
    });
    setLeaderboardState();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("requests leaderboard data for the current user and renders the backend pinned row", () => {
    setLeaderboardState({
      entries: [
        leaderboardEntry({
          originalRank: 1,
          stats: {
            points: { perps: "80", referral: "10", vault: "10" },
            realized_pnl: "25",
            volume: "1000",
          },
          userIndex: 2,
          username: "bob",
        }),
        leaderboardEntry({
          originalRank: 2,
          stats: {
            points: { perps: "30", referral: "10", vault: "10" },
            realized_pnl: "-5",
            volume: "500",
          },
          userIndex: 9,
          username: "user_9",
        }),
      ],
      pinnedUser: leaderboardEntry({
        originalRank: 42,
        stats: {
          points: { perps: "100", referral: "25", vault: "75" },
          realized_pnl: "-25",
          volume: "1000",
        },
        userIndex: 7,
        username: "alice",
      }),
    });

    render(<LeaderboardTable />);

    expect(leaderboardMocks.useLeaderboard).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 7,
    });

    const rows = bodyRows();
    expect(rows).toHaveLength(3);
    expect(rowText(rows[0])).toContain("42");
    expect(rowText(rows[0])).toContain(`alice ${m["points.leaderboard.you"]()}`);
    expect(rowText(rows[0])).toContain("$1,000");
    expect(rowText(rows[0])).toContain("-$25");
    expect(rowText(rows[0])).toContain(`200 ${m["points.header.points"]()}`);
    expect(rowText(rows[1])).toContain("bob");
    expect(rowText(rows[2])).toContain("User #9");
  });

  it("uses local user point totals as the pinned row when backend pinned data is absent", () => {
    setLeaderboardState({
      entries: [],
      pinnedUser: null,
    });

    render(<LeaderboardTable />);

    const rows = bodyRows();
    expect(rows).toHaveLength(1);
    expect(rowText(rows[0])).toContain("-");
    expect(rowText(rows[0])).toContain(`trader-seven ${m["points.leaderboard.you"]()}`);
    expect(rowText(rows[0])).toContain("$45,000");
    expect(rowText(rows[0])).toContain("$12");
    expect(rowText(rows[0])).toContain(`321 ${m["points.header.points"]()}`);
  });

  it("renders backend zero stats in the current user's pinned row", () => {
    setLeaderboardState({
      entries: [],
      pinnedUser: leaderboardEntry({
        originalRank: 99,
        stats: {
          points: { perps: "0", referral: "0", vault: "0" },
          realized_pnl: "0",
          volume: "0",
        },
        userIndex: 7,
        username: null,
      }),
    });

    render(<LeaderboardTable />);

    const rows = bodyRows();
    expect(rows).toHaveLength(1);
    expect(rowText(rows[0])).toContain("99");
    expect(rowText(rows[0])).toContain(`User #7 ${m["points.leaderboard.you"]()}`);
    expect(rowText(rows[0]).match(/\$0(?:\.00)?/g)).toHaveLength(2);
    expect(rowText(rows[0])).toContain(`0 ${m["points.header.points"]()}`);
  });

  it("preserves backend leaderboard entries with user index zero", () => {
    setLeaderboardState({
      entries: [
        leaderboardEntry({
          originalRank: 1,
          stats: {
            points: { perps: "5", referral: "3", vault: "2" },
            realized_pnl: "0",
            volume: "2500",
          },
          userIndex: 0,
          username: null,
        }),
      ],
      pinnedUser: null,
    });

    render(<LeaderboardTable />);

    const rows = bodyRows();
    expect(rows).toHaveLength(2);
    expect(rowText(rows[1])).toContain("1");
    expect(rowText(rows[1])).toContain("User #0");
    expect(rowText(rows[1])).toContain("$2,500");
    expect(rowText(rows[1])).toContain(`10 ${m["points.header.points"]()}`);
  });

  it("wires timeframe, sorting, and pagination controls to leaderboard callbacks", () => {
    setLeaderboardState({
      entries: [
        leaderboardEntry({
          originalRank: 1,
          userIndex: 2,
          username: "bob",
        }),
      ],
      page: 1,
      sort: "points",
      sortDirection: "desc",
      timeframe: null,
      totalPages: 3,
    });

    render(<LeaderboardTable />);

    fireEvent.click(
      screen.getByRole("button", { name: m["points.leaderboard.timeframes.oneWeek"]() }),
    );
    expect(leaderboardMocks.handleTimeframeChange).toHaveBeenCalledWith(1);

    fireEvent.click(
      screen.getByRole("button", { name: m["points.leaderboard.timeframes.oneMonth"]() }),
    );
    expect(leaderboardMocks.handleTimeframeChange).toHaveBeenCalledWith(4);

    fireEvent.click(screen.getByRole("button", { name: m["points.leaderboard.columns.volume"]() }));
    expect(leaderboardMocks.handleSortChange).toHaveBeenCalledWith("volume");

    fireEvent.click(screen.getByRole("button", { name: m["points.leaderboard.columns.pnl"]() }));
    expect(leaderboardMocks.handleSortChange).toHaveBeenCalledWith("pnl");

    fireEvent.click(screen.getByRole("button", { name: m["points.leaderboard.columns.points"]() }));
    expect(leaderboardMocks.handleSortChange).toHaveBeenCalledWith("points");

    fireEvent.click(screen.getByRole("button", { name: "2" }));
    expect(leaderboardMocks.setPage).toHaveBeenCalledWith(2);
  });

  it("shows an empty table state when there is no account and no leaderboard data", () => {
    leaderboardMocks.useAccount.mockReturnValue({
      userIndex: undefined,
      username: undefined,
    });
    setLeaderboardState({
      entries: [],
      pinnedUser: null,
    });

    render(<LeaderboardTable />);

    expect(screen.getByText("No data available")).toBeInTheDocument();
    expect(screen.queryAllByRole("row")).toHaveLength(1);
    expect(screen.queryByRole("button", { name: "2" })).not.toBeInTheDocument();
  });
});
