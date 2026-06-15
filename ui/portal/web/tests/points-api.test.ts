import { afterEach, describe, expect, it, vi } from "vitest";

import {
  fetchBoosters,
  fetchCurrentEpoch,
  fetchEpochPoints,
  fetchHuntedLatest,
  fetchLeaderboard,
  fetchPointsConfig,
  fetchTotalUsersWithPoints,
  fetchUserBoxes,
  fetchUserStats,
  openBoxes,
} from "../../../store/src/hooks/pointsApi";

function mockFetchJson(data: unknown, init: { ok?: boolean; status?: number } = {}) {
  return vi.spyOn(globalThis, "fetch").mockResolvedValue({
    ok: init.ok ?? true,
    status: init.status ?? 200,
    json: vi.fn().mockResolvedValue(data),
  } as unknown as Response);
}

describe("points service API adapter", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("requests user stats from the backend user stats endpoint", async () => {
    const payload = {
      stats: {
        points: { vault: "1", perps: "2", referral: "3" },
        realized_pnl: "-4",
        volume: "100",
      },
      rank: 5,
    };
    const fetchSpy = mockFetchJson(payload);

    await expect(fetchUserStats("https://points.test", 7)).resolves.toEqual(payload);

    expect(fetchSpy).toHaveBeenCalledWith("https://points.test/stats/user/7");
  });

  it("throws status-rich errors for failed user stats requests", async () => {
    mockFetchJson({}, { ok: false, status: 503 });

    await expect(fetchUserStats("https://points.test", 7)).rejects.toThrow(
      "Failed to fetch user stats: 503",
    );
  });

  it("requests the backend total-users metric", async () => {
    const fetchSpy = mockFetchJson(42);

    await expect(fetchTotalUsersWithPoints("https://points.test")).resolves.toBe(42);

    expect(fetchSpy).toHaveBeenCalledWith("https://points.test/stats/total-users-with-points");
  });

  it("throws status-rich errors for failed total-users requests", async () => {
    mockFetchJson({}, { ok: false, status: 500 });

    await expect(fetchTotalUsersWithPoints("https://points.test")).rejects.toThrow(
      "Failed to fetch total users: 500",
    );
  });

  it("serializes epoch point filters as backend query parameters", async () => {
    const fetchSpy = mockFetchJson([]);

    await fetchEpochPoints("https://points.test", 7, {
      min: 3,
      max: 8,
      order: "desc",
    });

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/stats/user/7/epochs?min=3&max=8&order=desc",
    );
  });

  it("preserves zero-valued user indexes and epoch filters", async () => {
    const fetchSpy = mockFetchJson([]);

    await fetchEpochPoints("https://points.test", 0, {
      min: 0,
      max: 0,
      order: "asc",
    });

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/stats/user/0/epochs?min=0&max=0&order=asc",
    );
  });

  it("throws status-rich errors for failed epoch point requests", async () => {
    mockFetchJson({}, { ok: false, status: 504 });

    await expect(fetchEpochPoints("https://points.test", 7)).rejects.toThrow(
      "Failed to fetch epoch points: 504",
    );
  });

  it("serializes leaderboard sort and timeframe filters", async () => {
    const fetchSpy = mockFetchJson([]);

    await fetchLeaderboard("https://points.test", {
      sort: "volume",
      timeframe: 14,
    });

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/leaderboard?sort=volume&timeframe=14",
    );
  });

  it("preserves zero-valued user indexes, ranks, and leaderboard timeframes", async () => {
    const payload = {
      stats: {
        points: { vault: "0", perps: "0", referral: "0" },
        realized_pnl: "0",
        volume: "0",
      },
      rank: 0,
    };
    const fetchSpy = mockFetchJson(payload);

    await expect(fetchUserStats("https://points.test", 0)).resolves.toEqual(payload);
    await fetchUserBoxes("https://points.test", 0);
    await fetchBoosters("https://points.test", 0);
    await fetchLeaderboard("https://points.test", {
      timeframe: 0,
    });

    expect(fetchSpy).toHaveBeenNthCalledWith(1, "https://points.test/stats/user/0");
    expect(fetchSpy).toHaveBeenNthCalledWith(2, "https://points.test/boxes/0");
    expect(fetchSpy).toHaveBeenNthCalledWith(3, "https://points.test/boosters/0");
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      "https://points.test/leaderboard?timeframe=0",
    );
  });

  it("throws status-rich errors for failed leaderboard requests", async () => {
    mockFetchJson({}, { ok: false, status: 429 });

    await expect(fetchLeaderboard("https://points.test")).rejects.toThrow(
      "Failed to fetch leaderboard: 429",
    );
  });

  it("requests user boxes from the backend user boxes endpoint", async () => {
    const payload = {
      chests: {
        bronze: {
          "1": { total: 2, opened: 1 },
        },
      },
      hunted: [{ chest: "bronze", loot: "bronze_shell", epoch: 1, opened: false }],
    };
    const fetchSpy = mockFetchJson(payload);

    await expect(fetchUserBoxes("https://points.test", 7)).resolves.toEqual(payload);

    expect(fetchSpy).toHaveBeenCalledWith("https://points.test/boxes/7");
  });

  it("throws status-rich errors for failed user boxes requests", async () => {
    mockFetchJson({}, { ok: false, status: 404 });

    await expect(fetchUserBoxes("https://points.test", 7)).rejects.toThrow(
      "Failed to fetch boxes: 404",
    );
  });

  it("omits empty optional open-boxes payloads", async () => {
    const fetchSpy = mockFetchJson({ success: true });

    await openBoxes("https://points.test", 7, {});

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/boxes/open",
      expect.objectContaining({
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ user_index: 7 }),
      }),
    );
  });

  it("includes boxes and hunted loot when opening rewards", async () => {
    const fetchSpy = mockFetchJson({ success: true });

    await openBoxes(
      "https://points.test",
      7,
      { bronze: { "1": 2 } },
      [{ epoch: 5, loot: "pearl_dango" }],
    );

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/boxes/open",
      expect.objectContaining({
        body: JSON.stringify({
          user_index: 7,
          boxes: { bronze: { "1": 2 } },
          hunted: [{ epoch: 5, loot: "pearl_dango" }],
        }),
      }),
    );
  });

  it("omits explicitly empty hunted reward arrays while preserving box openings", async () => {
    const fetchSpy = mockFetchJson({ success: true });

    await openBoxes("https://points.test", 7, { silver: { rare: 1 } }, []);

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/boxes/open",
      expect.objectContaining({
        body: JSON.stringify({
          user_index: 7,
          boxes: { silver: { rare: 1 } },
        }),
      }),
    );
  });

  it("serializes hunted-only reward openings without empty box payloads", async () => {
    const fetchSpy = mockFetchJson({ success: true });

    await openBoxes("https://points.test", 7, undefined, [
      { epoch: 8, loot: "golden_shell" },
      { epoch: 9, loot: "bronze_shell" },
    ]);

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/boxes/open",
      expect.objectContaining({
        body: JSON.stringify({
          user_index: 7,
          hunted: [
            { epoch: 8, loot: "golden_shell" },
            { epoch: 9, loot: "bronze_shell" },
          ],
        }),
      }),
    );
  });

  it("preserves backend user index zero and epoch zero when opening hunted rewards", async () => {
    const fetchSpy = mockFetchJson({ success: true });

    await openBoxes("https://points.test", 0, undefined, [{ epoch: 0, loot: "bronze_shell" }]);

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/boxes/open",
      expect.objectContaining({
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          user_index: 0,
          hunted: [{ epoch: 0, loot: "bronze_shell" }],
        }),
      }),
    );
  });

  it("throws status-rich errors for failed open-boxes requests", async () => {
    mockFetchJson({}, { ok: false, status: 409 });

    await expect(openBoxes("https://points.test", 7, { bronze: { "1": 1 } })).rejects.toThrow(
      "Failed to open boxes: 409",
    );
  });

  it("uses the backend start_after cursor for hunted latest pagination", async () => {
    const fetchSpy = mockFetchJson([]);

    await fetchHuntedLatest("https://points.test", {
      limit: 20,
      start_after: 12,
    });

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/boxes/hunted/latest?limit=20&start_after=12",
    );
  });

  it("preserves zero-valued hunted latest pagination cursors", async () => {
    const fetchSpy = mockFetchJson([]);

    await fetchHuntedLatest("https://points.test", {
      limit: 20,
      start_after: 0,
    });

    expect(fetchSpy).toHaveBeenCalledWith(
      "https://points.test/boxes/hunted/latest?limit=20&start_after=0",
    );
  });

  it("throws status-rich errors for failed hunted latest requests", async () => {
    mockFetchJson({}, { ok: false, status: 503 });

    await expect(fetchHuntedLatest("https://points.test")).rejects.toThrow(
      "Failed to fetch hunted latest: 503",
    );
  });

  it("requests user boosters from the backend boosters endpoint", async () => {
    const payload = {
      hunted_loots: [{ loot: "pearl_dango", epoch: 4, multiplier: "1.5" }],
    };
    const fetchSpy = mockFetchJson(payload);

    await expect(fetchBoosters("https://points.test", 7)).resolves.toEqual(payload);

    expect(fetchSpy).toHaveBeenCalledWith("https://points.test/boosters/7");
  });

  it("throws status-rich errors for failed boosters requests", async () => {
    mockFetchJson({}, { ok: false, status: 502 });

    await expect(fetchBoosters("https://points.test", 7)).rejects.toThrow(
      "Failed to fetch boosters: 502",
    );
  });

  it("requests the current epoch from the backend epoch endpoint", async () => {
    const payload = { status: "active", current_epoch: 12, remaining: "3600" };
    const fetchSpy = mockFetchJson(payload);

    await expect(fetchCurrentEpoch("https://points.test")).resolves.toEqual(payload);

    expect(fetchSpy).toHaveBeenCalledWith("https://points.test/event/epoch");
  });

  it("preserves not-started epoch block zero targets", async () => {
    const payload = {
      starts_at: {
        block: 0,
      },
      status: "not_started",
    };
    const fetchSpy = mockFetchJson(payload);

    await expect(fetchCurrentEpoch("https://points.test")).resolves.toEqual(payload);

    expect(fetchSpy).toHaveBeenCalledWith("https://points.test/event/epoch");
  });

  it("throws status-rich errors for failed current epoch requests", async () => {
    mockFetchJson({}, { ok: false, status: 503 });

    await expect(fetchCurrentEpoch("https://points.test")).rejects.toThrow(
      "Failed to fetch current epoch: 503",
    );
  });

  it("requests points config from the backend config endpoint", async () => {
    const payload = {
      boost_config: {
        pair: {
          "BTCUSDC": {
            "1": "1.25",
          },
        },
      },
    };
    const fetchSpy = mockFetchJson(payload);

    await expect(fetchPointsConfig("https://points.test")).resolves.toEqual(payload);

    expect(fetchSpy).toHaveBeenCalledWith("https://points.test/config");
  });

  it("throws status-rich errors for failed points config requests", async () => {
    mockFetchJson({}, { ok: false, status: 500 });

    await expect(fetchPointsConfig("https://points.test")).rejects.toThrow(
      "Failed to fetch points config: 500",
    );
  });
});
