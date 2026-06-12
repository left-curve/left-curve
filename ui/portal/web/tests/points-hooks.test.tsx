import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useBoostedPairs } from "../../../store/src/hooks/useBoostedPairs";
import { useBoosters } from "../../../store/src/hooks/useBoosters";
import { useBoxes } from "../../../store/src/hooks/useBoxes";
import { useCurrentEpoch } from "../../../store/src/hooks/useCurrentEpoch";
import { useEpochPoints } from "../../../store/src/hooks/useEpochPoints";
import { useHuntedLatest, useHuntedMultipliers } from "../../../store/src/hooks/useHuntedLatest";
import { useLeaderboard } from "../../../store/src/hooks/useLeaderboard";
import { usePredictPoints } from "../../../store/src/hooks/usePredictPoints";
import { createQueryClientWrapper } from "./utils/query-client";

const pointsApiMocks = vi.hoisted(() => ({
  fetchBoosters: vi.fn(),
  fetchCurrentEpoch: vi.fn(),
  fetchEpochPoints: vi.fn(),
  fetchHuntedLatest: vi.fn(),
  fetchLeaderboard: vi.fn(),
  fetchPointsConfig: vi.fn(),
  fetchUserBoxes: vi.fn(),
}));

vi.mock("../../../store/src/hooks/pointsApi.js", async (importOriginal) => {
  const actual = await importOriginal<object>();
  return {
    ...actual,
    fetchBoosters: pointsApiMocks.fetchBoosters,
    fetchCurrentEpoch: pointsApiMocks.fetchCurrentEpoch,
    fetchEpochPoints: pointsApiMocks.fetchEpochPoints,
    fetchHuntedLatest: pointsApiMocks.fetchHuntedLatest,
    fetchLeaderboard: pointsApiMocks.fetchLeaderboard,
    fetchPointsConfig: pointsApiMocks.fetchPointsConfig,
    fetchUserBoxes: pointsApiMocks.fetchUserBoxes,
  };
});

class MockEventSource {
  static instances: MockEventSource[] = [];

  onerror: (() => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  close = vi.fn();

  constructor(readonly url: string) {
    MockEventSource.instances.push(this);
  }

  emit(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent);
  }

  emitRaw(data: string) {
    this.onmessage?.({ data } as MessageEvent);
  }
}

const pointsUrl = "https://points.test";

describe("points hooks", () => {
  beforeEach(() => {
    MockEventSource.instances = [];
    Object.defineProperty(globalThis, "EventSource", {
      configurable: true,
      value: MockEventSource,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("derives unopened boxes, hunted boxes, NFT counts, and estimated volume", async () => {
    pointsApiMocks.fetchUserBoxes.mockResolvedValue({
      chests: {
        bronze: {
          common: { opened: 2, total: 5 },
          rare: { opened: 1, total: 1 },
        },
        gold: {
          legendary: { opened: 0, total: 2 },
        },
      },
      hunted: [
        {
          chest: "bronze",
          epoch: 2,
          loot: "bronze_shell",
          opened: false,
        },
        {
          chest: "silver",
          epoch: 1,
          loot: "pearl_dango",
          opened: true,
        },
      ],
    });

    const { result } = renderHook(
      () =>
        useBoxes({
          pointsUrl,
          userIndex: 7,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(pointsApiMocks.fetchUserBoxes).toHaveBeenCalledWith(pointsUrl, 7);
    expect(result.current.unopenedBoxes).toEqual({
      bronze: {
        common: 3,
      },
      gold: {
        legendary: 2,
      },
    });
    expect(result.current.huntedBoxes).toEqual([
      {
        chest: "bronze",
        epoch: 2,
        loot: "bronze_shell",
        opened: false,
      },
    ]);
    expect(result.current.unopenedCounts).toEqual({
      bronze: 4,
      gold: 2,
    });
    expect(result.current.estimatedVolume).toBe(650000);
    expect(result.current.nfts).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          rarity: "common",
          quantity: 2,
        }),
        expect.objectContaining({
          rarity: "rare",
          quantity: 1,
        }),
        expect.objectContaining({
          rarity: "legendary",
          quantity: 0,
        }),
      ]),
    );
  });

  it("keeps reward inventory hooks idle without an enabled user index", () => {
    const disabledBoxes = renderHook(
      () =>
        useBoxes({
          enabled: false,
          pointsUrl,
          userIndex: 7,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    const missingUserBoxes = renderHook(
      () =>
        useBoxes({
          pointsUrl,
          userIndex: undefined,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    for (const boxes of [disabledBoxes.result.current, missingUserBoxes.result.current]) {
      expect(boxes.isLoading).toBe(false);
      expect(boxes.unopenedBoxes).toEqual({});
      expect(boxes.huntedBoxes).toEqual([]);
      expect(boxes.unopenedCounts).toEqual({});
      expect(boxes.estimatedVolume).toBe(0);
      expect(boxes.nfts).toEqual([
        expect.objectContaining({ quantity: 0, rarity: "common" }),
        expect.objectContaining({ quantity: 0, rarity: "uncommon" }),
        expect.objectContaining({ quantity: 0, rarity: "rare" }),
        expect.objectContaining({ quantity: 0, rarity: "epic" }),
        expect.objectContaining({ quantity: 0, rarity: "legendary" }),
        expect.objectContaining({ quantity: 0, rarity: "mythic" }),
      ]);
    }
    expect(pointsApiMocks.fetchUserBoxes).not.toHaveBeenCalled();

    const disabledBoosters = renderHook(
      () =>
        useBoosters({
          enabled: false,
          pointsUrl,
          userIndex: 7,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    const missingUserBoosters = renderHook(
      () =>
        useBoosters({
          pointsUrl,
          userIndex: undefined,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(disabledBoosters.result.current).toEqual({
      huntedBoosters: [],
      isLoading: false,
    });
    expect(missingUserBoosters.result.current).toEqual({
      huntedBoosters: [],
      isLoading: false,
    });
    expect(pointsApiMocks.fetchBoosters).not.toHaveBeenCalled();
  });

  it("sorts hunted boosters by newest epoch and highest tier", async () => {
    pointsApiMocks.fetchBoosters.mockResolvedValue({
      hunted_loots: [
        { epoch: 4, loot: "bronze_shell", multiplier: "1.100000" },
        { epoch: 5, loot: "silver_shell", multiplier: "1.200000" },
        { epoch: 5, loot: "pearl_dango", multiplier: "2.000000" },
      ],
    });

    const { result } = renderHook(
      () =>
        useBoosters({
          pointsUrl,
          userIndex: 7,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(pointsApiMocks.fetchBoosters).toHaveBeenCalledWith(pointsUrl, 7);
    expect(result.current.huntedBoosters).toEqual([
      { epoch: 5, loot: "pearl_dango", multiplier: "2.000000", rank: 3 },
      { epoch: 5, loot: "silver_shell", multiplier: "1.200000", rank: 1 },
      { epoch: 4, loot: "bronze_shell", multiplier: "1.100000", rank: 0 },
    ]);
  });

  it("filters boosted pairs for the current epoch and ignores malformed or non-boost ranges", async () => {
    pointsApiMocks.fetchPointsConfig.mockResolvedValue({
      boost_config: {
        pair: {
          "perp/btcusd": {
            "1-3": "1.000000",
            "4-": "2.500000",
          },
          "perp/ethusd": {
            "5": "1.500000",
          },
          "perp/solusd": {
            malformed: "9.000000",
          },
        },
      },
    });

    const { result } = renderHook(
      () =>
        useBoostedPairs({
          currentEpoch: 5,
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.boostByPairId).toEqual({
        "perp/btcusd": "2.500000",
        "perp/ethusd": "1.500000",
      }),
    );

    expect(pointsApiMocks.fetchPointsConfig).toHaveBeenCalledWith(pointsUrl);

    const notStarted = renderHook(
      () =>
        useBoostedPairs({
          currentEpoch: null,
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(notStarted.result.current.boostByPairId).toEqual({});
  });

  it("matches boosted pair ranges for backend epoch zero", async () => {
    pointsApiMocks.fetchPointsConfig.mockResolvedValue({
      boost_config: {
        pair: {
          "perp/atomusd": {
            "0": "1.750000",
          },
          "perp/btcusd": {
            "0-2": "2.250000",
          },
          "perp/ethusd": {
            "1-": "3.000000",
          },
          "perp/solusd": {
            "0": "1.000000",
          },
        },
      },
    });

    const { result } = renderHook(
      () =>
        useBoostedPairs({
          currentEpoch: 0,
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.boostByPairId).toEqual({
        "perp/atomusd": "1.750000",
        "perp/btcusd": "2.250000",
      }),
    );

    expect(pointsApiMocks.fetchPointsConfig).toHaveBeenCalledWith(pointsUrl);
  });

  it("keeps backend-backed points views idle while disabled", () => {
    const currentEpoch = renderHook(
      () =>
        useCurrentEpoch({
          enabled: false,
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    expect(currentEpoch.result.current).toMatchObject({
      currentEpoch: null,
      endDate: null,
      isLoading: false,
      isStarted: false,
      startsAt: null,
    });

    const boostedPairs = renderHook(
      () =>
        useBoostedPairs({
          currentEpoch: 5,
          enabled: false,
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    expect(boostedPairs.result.current).toEqual({
      boostByPairId: {},
      isLoading: false,
    });

    const huntedMultipliers = renderHook(
      () =>
        useHuntedMultipliers({
          enabled: false,
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    expect(huntedMultipliers.result.current.isLoading).toBe(false);
    expect(huntedMultipliers.result.current.resolveMultiplier("bronze_shell", 5)).toBeNull();

    const huntedLatest = renderHook(
      () =>
        useHuntedLatest({
          enabled: false,
          limit: 2,
          pointsUrl,
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    expect(huntedLatest.result.current).toMatchObject({
      hasNextPage: false,
      isError: false,
      isFetching: false,
      isFetchingNextPage: false,
      isLoading: false,
      pages: [],
    });

    const leaderboard = renderHook(
      () =>
        useLeaderboard({
          enabled: false,
          pointsUrl,
          userIndex: 7,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );
    expect(leaderboard.result.current).toMatchObject({
      entries: [],
      isLoading: false,
      page: 1,
      pinnedUser: null,
      sort: "points",
      sortDirection: "desc",
      timeframe: null,
      totalPages: 0,
    });

    act(() => {
      leaderboard.result.current.handleSortChange("volume");
      leaderboard.result.current.handleTimeframeChange(2);
    });

    expect(pointsApiMocks.fetchCurrentEpoch).not.toHaveBeenCalled();
    expect(pointsApiMocks.fetchPointsConfig).not.toHaveBeenCalled();
    expect(pointsApiMocks.fetchHuntedLatest).not.toHaveBeenCalled();
    expect(pointsApiMocks.fetchLeaderboard).not.toHaveBeenCalled();
  });

  it("loads current epoch state and derives event timing", async () => {
    vi.spyOn(Date, "now").mockReturnValue(new Date("2026-06-08T12:00:00Z").getTime());
    pointsApiMocks.fetchCurrentEpoch.mockResolvedValue({
      current_epoch: 8,
      remaining: "30",
      status: "active",
    });

    const { result, rerender } = renderHook(
      ({ enabled }) =>
        useCurrentEpoch({
          enabled,
          pointsUrl,
        }),
      {
        initialProps: { enabled: true },
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isStarted).toBe(true));

    expect(pointsApiMocks.fetchCurrentEpoch).toHaveBeenCalledWith(pointsUrl);
    expect(result.current.currentEpoch).toBe(8);
    expect(result.current.endDate?.toISOString()).toBe("2026-06-08T12:00:30.000Z");
    expect(result.current.startsAt).toBeNull();

    pointsApiMocks.fetchCurrentEpoch.mockResolvedValueOnce({
      starts_at: {
        timestamp: "1790000000",
      },
      status: "not_started",
    });

    await act(async () => {
      await result.current.refetch();
    });
    rerender({ enabled: true });

    await waitFor(() =>
      expect(result.current).toMatchObject({
        currentEpoch: null,
        isStarted: false,
        startsAt: {
          timestamp: "1790000000",
        },
      }),
    );
  });

  it("treats active backend epoch zero as a started campaign", async () => {
    vi.spyOn(Date, "now").mockReturnValue(new Date("2026-06-08T12:00:00Z").getTime());
    pointsApiMocks.fetchCurrentEpoch.mockResolvedValue({
      current_epoch: 0,
      remaining: "5",
      status: "active",
    });

    const { result } = renderHook(
      () =>
        useCurrentEpoch({
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current).toMatchObject({
        currentEpoch: 0,
        isStarted: true,
        startsAt: null,
      }),
    );

    expect(result.current.endDate?.toISOString()).toBe("2026-06-08T12:00:05.000Z");
    expect(pointsApiMocks.fetchCurrentEpoch).toHaveBeenCalledWith(pointsUrl);
  });

  it("preserves block-based not-started epoch targets from the backend", async () => {
    pointsApiMocks.fetchCurrentEpoch.mockResolvedValue({
      starts_at: {
        block: 12345,
      },
      status: "not_started",
    });

    const { result } = renderHook(
      () =>
        useCurrentEpoch({
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current).toMatchObject({
        currentEpoch: null,
        endDate: null,
        isStarted: false,
        startsAt: {
          block: 12345,
        },
      }),
    );

    expect(pointsApiMocks.fetchCurrentEpoch).toHaveBeenCalledWith(pointsUrl);
  });

  it("preserves backend block zero as a not-started epoch target", async () => {
    pointsApiMocks.fetchCurrentEpoch.mockResolvedValue({
      starts_at: {
        block: 0,
      },
      status: "not_started",
    });

    const { result } = renderHook(
      () =>
        useCurrentEpoch({
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current).toMatchObject({
        currentEpoch: null,
        endDate: null,
        isStarted: false,
        startsAt: {
          block: 0,
        },
      }),
    );

    expect(pointsApiMocks.fetchCurrentEpoch).toHaveBeenCalledWith(pointsUrl);
  });

  it("queries epoch points with filters only when a user index is available", async () => {
    pointsApiMocks.fetchEpochPoints.mockResolvedValue([[4, { points: { perps: "2" } }]]);

    const { result } = renderHook(
      () =>
        useEpochPoints({
          max: 8,
          min: 2,
          order: "desc",
          pointsUrl,
          userIndex: 7,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.epochPoints).toEqual([[4, { points: { perps: "2" } }]]),
    );
    expect(pointsApiMocks.fetchEpochPoints).toHaveBeenCalledWith(pointsUrl, 7, {
      max: 8,
      min: 2,
      order: "desc",
    });

    cleanup();
    vi.clearAllMocks();

    renderHook(
      () =>
        useEpochPoints({
          pointsUrl,
          userIndex: undefined,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(pointsApiMocks.fetchEpochPoints).not.toHaveBeenCalled();
  });

  it("refreshes epoch points when the selected user or filter window changes", async () => {
    pointsApiMocks.fetchEpochPoints.mockImplementation(
      async (_pointsUrl: string, userIndex: number, filters: { min?: number; max?: number }) => [
        [
          filters.min ?? 0,
          {
            points: {
              perps: `${userIndex}-${filters.max ?? "none"}`,
            },
          },
        ],
      ],
    );

    const { result, rerender } = renderHook(
      ({ max, min, userIndex }) =>
        useEpochPoints({
          max,
          min,
          order: "asc",
          pointsUrl,
          userIndex,
        }),
      {
        initialProps: {
          max: 2,
          min: 1,
          userIndex: 7 as number | undefined,
        },
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.epochPoints).toEqual([[1, { points: { perps: "7-2" } }]]),
    );

    rerender({
      max: 4,
      min: 3,
      userIndex: 8,
    });

    await waitFor(() =>
      expect(result.current.epochPoints).toEqual([[3, { points: { perps: "8-4" } }]]),
    );

    expect(pointsApiMocks.fetchEpochPoints).toHaveBeenCalledWith(pointsUrl, 7, {
      max: 2,
      min: 1,
      order: "asc",
    });
    expect(pointsApiMocks.fetchEpochPoints).toHaveBeenCalledWith(pointsUrl, 8, {
      max: 4,
      min: 3,
      order: "asc",
    });
  });

  it("paginates hunted latest entries using the last block height as cursor", async () => {
    pointsApiMocks.fetchHuntedLatest
      .mockResolvedValueOnce([
        { block_height: 30, loot: "pearl_dango" },
        { block_height: 20, loot: "silver_shell" },
      ])
      .mockResolvedValueOnce([{ block_height: 10, loot: "bronze_shell" }]);

    const { result } = renderHook(
      () =>
        useHuntedLatest({
          limit: 2,
          pointsUrl,
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.pages).toHaveLength(1));
    expect(pointsApiMocks.fetchHuntedLatest).toHaveBeenCalledWith(pointsUrl, {
      limit: 2,
      start_after: undefined,
    });
    expect(result.current.hasNextPage).toBe(true);

    await act(async () => {
      await result.current.fetchNextPage();
    });

    expect(pointsApiMocks.fetchHuntedLatest).toHaveBeenCalledWith(pointsUrl, {
      limit: 2,
      start_after: 20,
    });
    await waitFor(() =>
      expect(result.current.pages).toEqual([
        [
          { block_height: 30, loot: "pearl_dango" },
          { block_height: 20, loot: "silver_shell" },
        ],
        [{ block_height: 10, loot: "bronze_shell" }],
      ]),
    );
  });

  it("preserves zero block-height cursors when paginating hunted latest entries", async () => {
    pointsApiMocks.fetchHuntedLatest
      .mockResolvedValueOnce([
        { block_height: 1, loot: "silver_shell" },
        { block_height: 0, loot: "bronze_shell" },
      ])
      .mockResolvedValueOnce([]);

    const { result } = renderHook(
      () =>
        useHuntedLatest({
          limit: 2,
          pointsUrl,
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.hasNextPage).toBe(true));

    await act(async () => {
      await result.current.fetchNextPage();
    });

    expect(pointsApiMocks.fetchHuntedLatest).toHaveBeenCalledWith(pointsUrl, {
      limit: 2,
      start_after: 0,
    });
    await waitFor(() =>
      expect(result.current.pages).toEqual([
        [
          { block_height: 1, loot: "silver_shell" },
          { block_height: 0, loot: "bronze_shell" },
        ],
        [],
      ]),
    );
  });

  it("stops hunted latest pagination when the backend returns fewer rows than requested", async () => {
    pointsApiMocks.fetchHuntedLatest.mockResolvedValueOnce([
      { block_height: 42, loot: "pearl_dango" },
    ]);

    const { result } = renderHook(
      () =>
        useHuntedLatest({
          limit: 2,
          pointsUrl,
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.pages).toEqual([[{ block_height: 42, loot: "pearl_dango" }]]),
    );

    expect(pointsApiMocks.fetchHuntedLatest).toHaveBeenCalledWith(pointsUrl, {
      limit: 2,
      start_after: undefined,
    });
    expect(result.current.hasNextPage).toBe(false);

    await act(async () => {
      await result.current.fetchNextPage();
    });

    expect(pointsApiMocks.fetchHuntedLatest).toHaveBeenCalledTimes(1);
    expect(result.current.pages).toEqual([[{ block_height: 42, loot: "pearl_dango" }]]);
  });

  it("surfaces hunted latest API failures without stale pages", async () => {
    const queryError = new Error("hunted latest unavailable");
    pointsApiMocks.fetchHuntedLatest.mockRejectedValueOnce(queryError);

    const { result } = renderHook(
      () =>
        useHuntedLatest({
          limit: 2,
          pointsUrl,
          refetchInterval: false,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(pointsApiMocks.fetchHuntedLatest).toHaveBeenCalledWith(pointsUrl, {
      limit: 2,
      start_after: undefined,
    });
    expect(result.current.pages).toEqual([]);
    expect(result.current.hasNextPage).toBe(false);
    expect(result.current.isFetching).toBe(false);
  });

  it("resolves hunted multipliers by closed, open-ended, single, and invalid epoch ranges", async () => {
    pointsApiMocks.fetchPointsConfig.mockResolvedValue({
      boost_config: {
        hunted: {
          bronze_shell: {
            "1-3": "1.250000",
            "4-": "2.000000",
          },
          silver_shell: {
            "6": "1.500000",
            "7": "not-a-decimal",
          },
          pearl_dango: {
            invalid: "9.000000",
          },
        },
      },
    });

    const { result } = renderHook(
      () =>
        useHuntedMultipliers({
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.resolveMultiplier("bronze_shell", 2)?.toString()).toBe("1.25");
    expect(result.current.resolveMultiplier("bronze_shell", 8)?.toString()).toBe("2");
    expect(result.current.resolveMultiplier("silver_shell", 6)?.toString()).toBe("1.5");
    expect(result.current.resolveMultiplier("silver_shell", 7)).toBeNull();
    expect(result.current.resolveMultiplier("pearl_dango", 8)).toBeNull();
    expect(result.current.resolveMultiplier("golden_shell", 8)).toBeNull();
  });

  it("resolves hunted multiplier ranges that start at backend epoch zero", async () => {
    pointsApiMocks.fetchPointsConfig.mockResolvedValue({
      boost_config: {
        hunted: {
          bronze_shell: {
            "0-2": "1.125000",
          },
          silver_shell: {
            "0": "1.500000",
          },
          golden_shell: {
            "0-": "2.250000",
          },
        },
      },
    });

    const { result } = renderHook(
      () =>
        useHuntedMultipliers({
          pointsUrl,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.resolveMultiplier("bronze_shell", 0)?.toString()).toBe("1.125");
    expect(result.current.resolveMultiplier("bronze_shell", 2)?.toString()).toBe("1.125");
    expect(result.current.resolveMultiplier("bronze_shell", 3)).toBeNull();
    expect(result.current.resolveMultiplier("silver_shell", 0)?.toString()).toBe("1.5");
    expect(result.current.resolveMultiplier("silver_shell", 1)).toBeNull();
    expect(result.current.resolveMultiplier("golden_shell", 0)?.toString()).toBe("2.25");
    expect(result.current.resolveMultiplier("golden_shell", 99)?.toString()).toBe("2.25");
  });

  it("paginates and re-sorts leaderboard entries while preserving original rank and pinned user", async () => {
    pointsApiMocks.fetchLeaderboard.mockResolvedValue(
      Array.from({ length: 12 }, (_, index) => ({
        user_index: index + 1,
        username: `user-${index + 1}`,
        points: `${1200 - index}`,
        volume: `${index}`,
      })),
    );

    const { result } = renderHook(
      () =>
        useLeaderboard({
          pointsUrl,
          userIndex: 12,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.entries).toHaveLength(10));

    expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledWith(pointsUrl, {
      sort: "points",
      timeframe: undefined,
    });
    expect(result.current.totalPages).toBe(2);
    expect(result.current.pinnedUser).toMatchObject({
      originalRank: 12,
      user_index: 12,
    });

    act(() => {
      result.current.setPage(2);
    });

    expect(result.current.entries.map((entry) => entry.user_index)).toEqual([11, 12]);

    act(() => {
      result.current.handleSortChange("points");
    });

    expect(result.current.page).toBe(1);
    expect(result.current.sortDirection).toBe("asc");
    expect(result.current.entries.map((entry) => entry.user_index).slice(0, 3)).toEqual([
      12, 11, 10,
    ]);

    act(() => {
      result.current.handleTimeframeChange(2);
    });

    await waitFor(() =>
      expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledWith(pointsUrl, {
        sort: "points",
        timeframe: 2,
      }),
    );
    expect(result.current.page).toBe(1);
  });

  it("requests a new leaderboard backend sort when switching columns", async () => {
    pointsApiMocks.fetchLeaderboard.mockResolvedValue(
      Array.from({ length: 3 }, (_, index) => ({
        user_index: index + 1,
        username: `user-${index + 1}`,
        points: `${100 - index}`,
        volume: `${index + 1}`,
      })),
    );

    const { result } = renderHook(
      () =>
        useLeaderboard({
          pointsUrl,
          userIndex: 2,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledWith(pointsUrl, {
        sort: "points",
        timeframe: undefined,
      }),
    );

    act(() => {
      result.current.setPage(2);
      result.current.handleTimeframeChange(4);
    });

    await waitFor(() =>
      expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledWith(pointsUrl, {
        sort: "points",
        timeframe: 4,
      }),
    );

    act(() => {
      result.current.handleSortChange("volume");
    });

    await waitFor(() =>
      expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledWith(pointsUrl, {
        sort: "volume",
        timeframe: 4,
      }),
    );

    expect(result.current).toMatchObject({
      page: 1,
      sort: "volume",
      sortDirection: "desc",
      timeframe: 4,
    });
  });

  it("updates the pinned leaderboard user when the selected account changes", async () => {
    pointsApiMocks.fetchLeaderboard.mockResolvedValue([
      {
        points: "900",
        user_index: 7,
        username: "spot-trader",
        volume: "120",
      },
      {
        points: "800",
        user_index: 8,
        username: "vault-trader",
        volume: "110",
      },
      {
        points: "700",
        user_index: 9,
        username: "perps-trader",
        volume: "100",
      },
    ]);

    const { result, rerender } = renderHook(
      ({ userIndex }) =>
        useLeaderboard({
          pointsUrl,
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
      expect(result.current.pinnedUser).toMatchObject({
        originalRank: 1,
        user_index: 7,
      }),
    );
    expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledTimes(1);

    rerender({ userIndex: 8 });

    await waitFor(() =>
      expect(result.current.pinnedUser).toMatchObject({
        originalRank: 2,
        user_index: 8,
      }),
    );
    expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledTimes(1);
    expect(result.current.entries.map((entry) => entry.user_index)).toEqual([7, 8, 9]);

    rerender({ userIndex: undefined });

    expect(result.current.pinnedUser).toBeNull();
    expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledTimes(1);
  });

  it("pins leaderboard entries for backend user index zero", async () => {
    pointsApiMocks.fetchLeaderboard.mockResolvedValue([
      {
        points: "1000",
        user_index: 0,
        username: "genesis-trader",
        volume: "0",
      },
      {
        points: "900",
        user_index: 1,
        username: "second-trader",
        volume: "100",
      },
    ]);

    const { result } = renderHook(
      () =>
        useLeaderboard({
          pointsUrl,
          userIndex: 0,
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.pinnedUser).toMatchObject({
        originalRank: 1,
        user_index: 0,
        username: "genesis-trader",
      }),
    );

    expect(result.current.entries.map((entry) => entry.user_index)).toEqual([0, 1]);
    expect(pointsApiMocks.fetchLeaderboard).toHaveBeenCalledWith(pointsUrl, {
      sort: "points",
      timeframe: undefined,
    });
  });

  it("streams predicted points over EventSource and closes on malformed/error paths", async () => {
    const { result, unmount } = renderHook(() =>
      usePredictPoints({
        pointsUrl,
        userIndex: 7,
      }),
    );

    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].url).toBe(`${pointsUrl}/predict/points/7`);

    act(() => {
      MockEventSource.instances[0].emitRaw("not json");
    });
    expect(result.current.predictedPoints).toBeNull();

    act(() => {
      MockEventSource.instances[0].emit({
        points: "12.5",
        rank: 4,
      });
    });

    expect(result.current.predictedPoints).toEqual({
      points: "12.5",
      rank: 4,
    });

    act(() => {
      MockEventSource.instances[0].onerror?.();
    });

    expect(MockEventSource.instances[0].close).toHaveBeenCalledOnce();
    unmount();
    expect(MockEventSource.instances[0].close).toHaveBeenCalledTimes(2);

    cleanup();
    MockEventSource.instances = [];

    renderHook(() =>
      usePredictPoints({
        pointsUrl,
        userIndex: undefined,
      }),
    );

    expect(MockEventSource.instances).toHaveLength(0);
  });

  it("opens predicted point streams for backend user index zero", () => {
    const { result, unmount } = renderHook(() =>
      usePredictPoints({
        pointsUrl,
        userIndex: 0,
      }),
    );

    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].url).toBe(`${pointsUrl}/predict/points/0`);

    act(() => {
      MockEventSource.instances[0].emit({
        points: "0",
        rank: 0,
      });
    });

    expect(result.current.predictedPoints).toEqual({
      points: "0",
      rank: 0,
    });

    unmount();
    expect(MockEventSource.instances[0].close).toHaveBeenCalledOnce();
  });

  it("opens and closes predicted point streams as user context changes", () => {
    const { rerender, unmount } = renderHook(
      ({ enabled, userIndex }) =>
        usePredictPoints({
          enabled,
          pointsUrl,
          userIndex,
        }),
      {
        initialProps: {
          enabled: false,
          userIndex: 7 as number | undefined,
        },
      },
    );

    expect(MockEventSource.instances).toHaveLength(0);

    rerender({
      enabled: true,
      userIndex: 7,
    });

    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].url).toBe(`${pointsUrl}/predict/points/7`);

    rerender({
      enabled: false,
      userIndex: 7,
    });

    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].close).toHaveBeenCalledOnce();

    rerender({
      enabled: true,
      userIndex: 8,
    });

    expect(MockEventSource.instances).toHaveLength(2);
    expect(MockEventSource.instances[1].url).toBe(`${pointsUrl}/predict/points/8`);

    rerender({
      enabled: true,
      userIndex: undefined,
    });

    expect(MockEventSource.instances).toHaveLength(2);
    expect(MockEventSource.instances[1].close).toHaveBeenCalledOnce();

    unmount();
    expect(MockEventSource.instances[1].close).toHaveBeenCalledOnce();
  });
});
