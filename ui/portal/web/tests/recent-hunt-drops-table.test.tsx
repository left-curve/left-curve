import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { Decimal } from "@left-curve/utils";

import { RecentHuntDropsTable } from "../src/components/points/leaderboard/RecentHuntDropsTable";

const recentDropsMocks = vi.hoisted(() => ({
  fetchNextPage: vi.fn(),
  resolveMultiplier: vi.fn(),
  useHuntedLatest: vi.fn(),
  useHuntedMultipliers: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useHuntedLatest: recentDropsMocks.useHuntedLatest,
  useHuntedMultipliers: recentDropsMocks.useHuntedMultipliers,
}));

type HuntedDropFixture = {
  blockHeight?: number;
  epoch?: number;
  loot: "bronze_shell" | "golden_shell" | "pearl_dango" | "silver_shell";
  timestampSeconds?: number;
  userIndex: number;
  username: string | null;
};

function huntedDrop({
  blockHeight = 1234,
  epoch = 9,
  loot,
  timestampSeconds = 1_749_384_000,
  userIndex,
  username,
}: HuntedDropFixture) {
  return {
    block_height: blockHeight,
    block_timestamp: String(timestampSeconds),
    epoch,
    loot,
    user_index: userIndex,
    username,
  };
}

function setLatestDrops({
  hasNextPage = false,
  isFetching = false,
  isFetchingNextPage = false,
  isLoading = false,
  pages = [],
}: Partial<{
  hasNextPage: boolean;
  isFetching: boolean;
  isFetchingNextPage: boolean;
  isLoading: boolean;
  pages: unknown[][];
}> = {}) {
  recentDropsMocks.useHuntedLatest.mockReturnValue({
    fetchNextPage: recentDropsMocks.fetchNextPage,
    hasNextPage,
    isFetching,
    isFetchingNextPage,
    isLoading,
    pages,
  });
}

function setMultipliers(
  multipliers: Partial<
    Record<HuntedDropFixture["loot"], Partial<Record<number, InstanceType<typeof Decimal> | null>>>
  > = {},
) {
  recentDropsMocks.resolveMultiplier.mockImplementation(
    (loot: HuntedDropFixture["loot"], epoch: number) => multipliers[loot]?.[epoch] ?? null,
  );
  recentDropsMocks.useHuntedMultipliers.mockReturnValue({
    resolveMultiplier: recentDropsMocks.resolveMultiplier,
  });
}

function bodyRows() {
  return screen.getAllByRole("row").slice(1);
}

function rowText(row: HTMLElement) {
  return row.textContent ?? "";
}

describe("RecentHuntDropsTable", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-08T12:00:00Z"));
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          pointsUrl: "https://points.example",
        },
      },
    });
    recentDropsMocks.fetchNextPage.mockResolvedValue(undefined);
    setLatestDrops();
    setMultipliers();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("requests recent drops and renders reward labels, usernames, boost percentages, and relative time", () => {
    setLatestDrops({
      pages: [
        [
          huntedDrop({
            epoch: 9,
            loot: "pearl_dango",
            timestampSeconds: Date.parse("2026-06-07T12:00:00Z") / 1000,
            userIndex: 7,
            username: "alice",
          }),
          huntedDrop({
            epoch: 8,
            loot: "bronze_shell",
            timestampSeconds: Date.parse("2026-06-07T11:00:00Z") / 1000,
            userIndex: 9,
            username: "user_9",
          }),
          huntedDrop({
            epoch: 8,
            loot: "golden_shell",
            timestampSeconds: Date.parse("2026-06-07T10:00:00Z") / 1000,
            userIndex: 10,
            username: null,
          }),
        ],
      ],
    });
    setMultipliers({
      bronze_shell: {
        8: Decimal("1.2"),
      },
      pearl_dango: {
        9: Decimal("2"),
      },
    });

    render(<RecentHuntDropsTable />);

    expect(recentDropsMocks.useHuntedLatest).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
    });
    expect(recentDropsMocks.useHuntedMultipliers).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
    });

    const rows = bodyRows();
    expect(rows).toHaveLength(3);
    expect(rowText(rows[0])).toContain("alice");
    expect(rowText(rows[0])).toContain(m["points.leaderboard.recentDrops.rewards.pearl_dango"]());
    expect(rowText(rows[0])).toContain("+100%");
    expect(rowText(rows[0])).toContain("1 day ago");
    expect(rowText(rows[1])).toContain("User #9");
    expect(rowText(rows[1])).toContain(m["points.leaderboard.recentDrops.rewards.bronze_shell"]());
    expect(rowText(rows[1])).toContain("+20%");
    expect(rowText(rows[2])).toContain("User #10");
    expect(rowText(rows[2])).toContain("—");
  });

  it("preserves backend user index and epoch zero in recent drops", () => {
    setLatestDrops({
      pages: [
        [
          huntedDrop({
            epoch: 0,
            loot: "silver_shell",
            timestampSeconds: Date.parse("2026-06-08T11:55:00Z") / 1000,
            userIndex: 0,
            username: null,
          }),
        ],
      ],
    });
    setMultipliers({
      silver_shell: {
        0: Decimal("1.25"),
      },
    });

    render(<RecentHuntDropsTable />);

    const [row] = bodyRows();
    expect(rowText(row)).toContain("User #0");
    expect(rowText(row)).toContain(m["points.leaderboard.recentDrops.rewards.silver_shell"]());
    expect(rowText(row)).toContain("+25%");
    expect(recentDropsMocks.resolveMultiplier).toHaveBeenCalledWith("silver_shell", 0);
  });

  it("renders backend no-boost multipliers as zero percent instead of missing data", () => {
    setLatestDrops({
      pages: [
        [
          huntedDrop({
            epoch: 12,
            loot: "bronze_shell",
            timestampSeconds: Date.parse("2026-06-08T11:50:00Z") / 1000,
            userIndex: 11,
            username: "steady",
          }),
          huntedDrop({
            epoch: 12,
            loot: "golden_shell",
            timestampSeconds: Date.parse("2026-06-08T11:45:00Z") / 1000,
            userIndex: 12,
            username: "missing",
          }),
        ],
      ],
    });
    setMultipliers({
      bronze_shell: {
        12: Decimal("1"),
      },
    });

    render(<RecentHuntDropsTable />);

    const rows = bodyRows();
    expect(rowText(rows[0])).toContain("steady");
    expect(rowText(rows[0])).toContain("+0%");
    expect(rowText(rows[1])).toContain("missing");
    expect(rowText(rows[1])).toContain("—");
  });

  it("paginates displayed drops and fetches the backend overflow page from the last local page", async () => {
    setLatestDrops({
      hasNextPage: true,
      pages: [
        Array.from({ length: 6 }, (_, index) =>
          huntedDrop({
            blockHeight: 200 - index,
            epoch: 4,
            loot: index === 5 ? "silver_shell" : "bronze_shell",
            timestampSeconds: 1_749_384_000 - index * 60,
            userIndex: index + 1,
            username: `hunter-${index + 1}`,
          }),
        ),
      ],
    });

    render(<RecentHuntDropsTable />);

    expect(bodyRows()).toHaveLength(5);
    expect(rowText(bodyRows()[0])).toContain("hunter-1");
    expect(rowText(bodyRows()[4])).toContain("hunter-5");

    fireEvent.click(screen.getByRole("button", { name: "2" }));

    expect(bodyRows()).toHaveLength(1);
    expect(rowText(bodyRows()[0])).toContain("hunter-6");

    const nextButton = screen.getAllByRole("button").at(-1);
    expect(nextButton).toBeDefined();
    await act(async () => {
      fireEvent.click(nextButton!);
    });

    expect(recentDropsMocks.fetchNextPage).toHaveBeenCalledOnce();
  });

  it("flattens already-fetched backend pages before applying local display pagination", () => {
    setLatestDrops({
      pages: [
        Array.from({ length: 5 }, (_, index) =>
          huntedDrop({
            blockHeight: 300 - index,
            epoch: 7,
            loot: "bronze_shell",
            timestampSeconds: 1_749_384_000 - index * 60,
            userIndex: index + 1,
            username: `first-page-${index + 1}`,
          }),
        ),
        [
          huntedDrop({
            blockHeight: 250,
            epoch: 8,
            loot: "pearl_dango",
            timestampSeconds: 1_749_383_000,
            userIndex: 8,
            username: "second-page-1",
          }),
          huntedDrop({
            blockHeight: 249,
            epoch: 8,
            loot: "silver_shell",
            timestampSeconds: 1_749_382_000,
            userIndex: 9,
            username: "second-page-2",
          }),
        ],
      ],
    });

    render(<RecentHuntDropsTable />);

    expect(bodyRows()).toHaveLength(5);
    expect(rowText(bodyRows()[0])).toContain("first-page-1");

    fireEvent.click(screen.getByRole("button", { name: "2" }));

    expect(bodyRows()).toHaveLength(2);
    expect(rowText(bodyRows()[0])).toContain("second-page-1");
    expect(rowText(bodyRows()[1])).toContain("second-page-2");
    expect(recentDropsMocks.fetchNextPage).not.toHaveBeenCalled();
  });

  it("keeps the overflow next button disabled while the next backend page is loading", () => {
    setLatestDrops({
      hasNextPage: true,
      isFetchingNextPage: true,
      pages: [
        [
          huntedDrop({
            loot: "silver_shell",
            userIndex: 7,
            username: "alice",
          }),
        ],
      ],
    });

    render(<RecentHuntDropsTable />);

    const nextButton = screen.getAllByRole("button").at(-1);
    expect(nextButton).toBeDisabled();
  });

  it("renders loading and empty states from backend query state", () => {
    setLatestDrops({
      isLoading: true,
      pages: [],
    });

    const { rerender } = render(<RecentHuntDropsTable />);

    expect(screen.queryByText(m["points.leaderboard.recentDrops.empty"]())).not.toBeInTheDocument();
    expect(screen.getAllByRole("row")).toHaveLength(4);

    setLatestDrops({
      isLoading: false,
      pages: [],
    });
    rerender(<RecentHuntDropsTable />);

    expect(screen.getByText(m["points.leaderboard.recentDrops.empty"]())).toBeInTheDocument();
  });
});
