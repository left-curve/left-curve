import { cleanup, render, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { usePoints } from "../../../store/src/hooks/usePoints";
import { createQueryClientWrapper, createTestQueryClient } from "./utils/query-client";

const pointsApiMocks = vi.hoisted(() => ({
  fetchTotalUsersWithPoints: vi.fn(),
  fetchUserStats: vi.fn(),
}));

vi.mock("../../../store/src/hooks/pointsApi.js", async (importOriginal) => {
  const actual = await importOriginal<object>();
  return {
    ...actual,
    fetchTotalUsersWithPoints: pointsApiMocks.fetchTotalUsersWithPoints,
    fetchUserStats: pointsApiMocks.fetchUserStats,
  };
});

function renderUsePoints(parameters: Parameters<typeof usePoints>[0]) {
  const queryClient = createTestQueryClient();

  function Consumer() {
    const points = usePoints(parameters);
    return <pre data-testid="points">{JSON.stringify(points)}</pre>;
  }

  render(<Consumer />, { wrapper: createQueryClientWrapper(queryClient) });

  return queryClient;
}

function readPoints() {
  return JSON.parse(screen.getByTestId("points").textContent ?? "{}") as Record<string, unknown>;
}

describe("usePoints", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("derives dashboard point totals from backend user stats", async () => {
    pointsApiMocks.fetchUserStats.mockResolvedValue({
      stats: {
        points: {
          vault: "1.5",
          perps: "2.25",
          referral: "3.75",
        },
        realized_pnl: "-12.25",
        volume: "1000.5",
      },
      rank: 5,
      compensation: {
        vault: "9",
        unrealized: "10",
      },
    });
    pointsApiMocks.fetchTotalUsersWithPoints.mockResolvedValue(25);

    renderUsePoints({
      pointsUrl: "https://points.test",
      userIndex: 7,
    });

    await waitFor(() => expect(readPoints().isLoading).toBe(false));

    expect(pointsApiMocks.fetchUserStats).toHaveBeenCalledWith("https://points.test", 7);
    expect(pointsApiMocks.fetchTotalUsersWithPoints).toHaveBeenCalledWith("https://points.test");
    expect(readPoints()).toMatchObject({
      points: 7.5,
      lpPoints: 1.5,
      tradingPoints: 2.25,
      referralPoints: 3.75,
      volume: 1000.5,
      pnl: -12.25,
      rank: 5,
      percentile: 84,
      compensation: {
        vault: "9",
        unrealized: "10",
      },
      isLoading: false,
    });
  });

  it("returns zeroed derived values until backend stats are available", async () => {
    pointsApiMocks.fetchUserStats.mockResolvedValue({
      stats: {
        points: {
          vault: "0",
          perps: "0",
          referral: "0",
        },
        realized_pnl: "0",
        volume: "0",
      },
      rank: 0,
    });
    pointsApiMocks.fetchTotalUsersWithPoints.mockResolvedValue(0);

    renderUsePoints({
      pointsUrl: "https://points.test",
      userIndex: 7,
    });

    await waitFor(() => expect(readPoints().isLoading).toBe(false));

    expect(readPoints()).toMatchObject({
      points: 0,
      lpPoints: 0,
      tradingPoints: 0,
      referralPoints: 0,
      volume: 0,
      pnl: 0,
      rank: 0,
      percentile: 0,
      isLoading: false,
    });
  });

  it("does not call points endpoints when disabled", () => {
    renderUsePoints({
      pointsUrl: "https://points.test",
      userIndex: 7,
      enabled: false,
    });

    expect(readPoints()).toMatchObject({
      points: 0,
      lpPoints: 0,
      tradingPoints: 0,
      referralPoints: 0,
      volume: 0,
      pnl: 0,
      rank: 0,
      percentile: 0,
      isLoading: false,
    });
    expect(pointsApiMocks.fetchUserStats).not.toHaveBeenCalled();
    expect(pointsApiMocks.fetchTotalUsersWithPoints).not.toHaveBeenCalled();
  });

  it("does not request user stats when the account has no user index", async () => {
    pointsApiMocks.fetchTotalUsersWithPoints.mockResolvedValue(25);

    renderUsePoints({
      pointsUrl: "https://points.test",
      userIndex: undefined,
    });

    await waitFor(() => expect(readPoints().isLoading).toBe(false));

    expect(pointsApiMocks.fetchUserStats).not.toHaveBeenCalled();
    expect(pointsApiMocks.fetchTotalUsersWithPoints).toHaveBeenCalledWith("https://points.test");
    expect(readPoints()).toMatchObject({
      points: 0,
      lpPoints: 0,
      tradingPoints: 0,
      referralPoints: 0,
      volume: 0,
      pnl: 0,
      rank: 0,
      percentile: 0,
      isLoading: false,
    });
  });

  it("refreshes backend user stats when the selected user index changes", async () => {
    pointsApiMocks.fetchUserStats.mockImplementation(
      async (_pointsUrl: string, userIndex: number) => ({
        rank: userIndex === 7 ? 10 : 4,
        stats: {
          points: {
            perps: userIndex === 7 ? "2" : "8",
            referral: userIndex === 7 ? "3" : "9",
            vault: userIndex === 7 ? "1" : "7",
          },
          realized_pnl: userIndex === 7 ? "11" : "44",
          volume: userIndex === 7 ? "100" : "400",
        },
      }),
    );
    pointsApiMocks.fetchTotalUsersWithPoints.mockResolvedValue(20);

    const { result, rerender } = renderHook(
      ({ userIndex }) =>
        usePoints({
          pointsUrl: "https://points.test",
          userIndex,
        }),
      {
        initialProps: {
          userIndex: 7 as number | undefined,
        },
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current).toMatchObject({
        points: 6,
        rank: 10,
        volume: 100,
      }),
    );
    expect(result.current.percentile).toBeCloseTo(55);

    rerender({ userIndex: 8 });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        points: 24,
        rank: 4,
        volume: 400,
      }),
    );
    expect(result.current.percentile).toBeCloseTo(85);

    expect(pointsApiMocks.fetchUserStats).toHaveBeenCalledWith("https://points.test", 7);
    expect(pointsApiMocks.fetchUserStats).toHaveBeenCalledWith("https://points.test", 8);
    expect(pointsApiMocks.fetchTotalUsersWithPoints).toHaveBeenCalledTimes(1);
  });
});
