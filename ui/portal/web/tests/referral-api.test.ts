import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  queryCommissionRateOverride,
  queryRefereeStats,
  queryReferralData,
  queryReferralParams,
  queryReferralSettings,
  queryReferrer,
  queryVolume,
} from "../../../store/src/hooks/referralApi";

import type { PublicClient } from "@left-curve/sdk";

function createClient(response: unknown) {
  const queryApp = vi.fn(async () => response);
  return {
    client: { queryApp } as unknown as PublicClient,
    queryApp,
  };
}

function createRejectingClient(error: unknown) {
  const queryApp = vi.fn(async () => {
    throw error;
  });
  return {
    client: { queryApp } as unknown as PublicClient,
    queryApp,
  };
}

const perpsAddress = "0x7065727073000000000000000000000000000000";

describe("referral contract query adapter", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("serializes volume queries with the perps contract address and string since value", async () => {
    const { client, queryApp } = createClient({ wasm_smart: "12345.67" });

    await expect(
      queryVolume(client, perpsAddress, "0x7472616465720000000000000000000000000000", 1710000000),
    ).resolves.toBe("12345.67");

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            volume: {
              user: "0x7472616465720000000000000000000000000000",
              since: "1710000000",
            },
          },
        },
      },
    });
  });

  it("omits the optional since value for lifetime volume queries", async () => {
    const { client, queryApp } = createClient({ wasm_smart: "98765.43" });

    await expect(
      queryVolume(client, perpsAddress, "0x7472616465720000000000000000000000000000"),
    ).resolves.toBe("98765.43");

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            volume: {
              user: "0x7472616465720000000000000000000000000000",
            },
          },
        },
      },
    });
  });

  it("preserves zero-valued volume since filters", async () => {
    const { client, queryApp } = createClient({ wasm_smart: "0" });

    await expect(
      queryVolume(client, perpsAddress, "0x7472616465720000000000000000000000000000", 0),
    ).resolves.toBe("0");

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            volume: {
              user: "0x7472616465720000000000000000000000000000",
              since: "0",
            },
          },
        },
      },
    });
  });

  it("keeps null referrers distinct from valid zero-like values", async () => {
    const { client } = createClient({ wasm_smart: null });

    await expect(queryReferrer(client, perpsAddress, 42)).resolves.toBeNull();
  });

  it("preserves zero-valued referee and referrer user indexes", async () => {
    const { client, queryApp } = createClient({ wasm_smart: 0 });

    await expect(queryReferrer(client, perpsAddress, 0)).resolves.toBe(0);

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referrer: {
              referee: 0,
            },
          },
        },
      },
    });
  });

  it("serializes referrer lookups by referee user index", async () => {
    const { client, queryApp } = createClient({ wasm_smart: 7 });

    await expect(queryReferrer(client, perpsAddress, 42)).resolves.toBe(7);

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referrer: {
              referee: 42,
            },
          },
        },
      },
    });
  });

  it("deserializes referral data fields from backend snake case to frontend camel case", async () => {
    const { client, queryApp } = createClient({
      wasm_smart: {
        volume: "9000",
        commission_shared_by_referrer: "10",
        referee_count: 3,
        referees_volume: "12000",
        commission_earned_from_referees: "44",
        cumulative_daily_active_referees: 8,
        cumulative_global_active_referees: 12,
      },
    });

    await expect(queryReferralData(client, perpsAddress, 7, 1710000001)).resolves.toEqual({
      volume: "9000",
      commissionSharedByReferrer: "10",
      refereeCount: 3,
      refereesVolume: "12000",
      commissionEarnedFromReferees: "44",
      cumulativeDailyActiveReferees: 8,
      cumulativeGlobalActiveReferees: 12,
    });

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referral_data: {
              user: 7,
              since: "1710000001",
            },
          },
        },
      },
    });
  });

  it("omits the optional since value for lifetime referral data queries", async () => {
    const { client, queryApp } = createClient({
      wasm_smart: {
        volume: "7000",
        commission_shared_by_referrer: "12",
        referee_count: 2,
        referees_volume: "7100",
        commission_earned_from_referees: "18",
        cumulative_daily_active_referees: 4,
        cumulative_global_active_referees: 5,
      },
    });

    await expect(queryReferralData(client, perpsAddress, 7)).resolves.toEqual({
      volume: "7000",
      commissionSharedByReferrer: "12",
      refereeCount: 2,
      refereesVolume: "7100",
      commissionEarnedFromReferees: "18",
      cumulativeDailyActiveReferees: 4,
      cumulativeGlobalActiveReferees: 5,
    });

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referral_data: {
              user: 7,
            },
          },
        },
      },
    });
  });

  it("preserves zero-valued referral users and since filters in backend queries", async () => {
    const { client, queryApp } = createClient({
      wasm_smart: {
        volume: "0",
        commission_shared_by_referrer: "0",
        referee_count: 0,
        referees_volume: "0",
        commission_earned_from_referees: "0",
        cumulative_daily_active_referees: 0,
        cumulative_global_active_referees: 0,
      },
    });

    await expect(queryReferralData(client, perpsAddress, 0, 0)).resolves.toEqual({
      volume: "0",
      commissionSharedByReferrer: "0",
      refereeCount: 0,
      refereesVolume: "0",
      commissionEarnedFromReferees: "0",
      cumulativeDailyActiveReferees: 0,
      cumulativeGlobalActiveReferees: 0,
    });

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referral_data: {
              user: 0,
              since: "0",
            },
          },
        },
      },
    });
  });

  it("flattens referrer-to-referee tuple responses into user-indexed rows", async () => {
    const orderBy = {
      order: "Descending" as const,
      limit: 25,
      index: { volume: { startAfter: "1000" } },
    };
    const { client, queryApp } = createClient({
      wasm_smart: [
        [
          12,
          {
            registered_at: 171,
            volume: "2000",
            commission_earned: "18",
            last_day_active: 193,
          },
        ],
      ],
    });

    await expect(queryRefereeStats(client, perpsAddress, 7, orderBy)).resolves.toEqual([
      {
        userIndex: 12,
        registeredAt: 171,
        volume: "2000",
        commissionEarned: "18",
        lastDayActive: 193,
      },
    ]);

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referrer_to_referee_stats: {
              referrer: 7,
              order_by: {
                order: "Descending",
                limit: 25,
                index: {
                  volume: {
                    start_after: "1000",
                  },
                },
              },
            },
          },
        },
      },
    });
  });

  it("preserves zero-valued referrer stats indexes and cursors", async () => {
    const orderBy = {
      order: "Ascending" as const,
      limit: 25,
      index: { volume: { startAfter: "0" } },
    };
    const { client, queryApp } = createClient({
      wasm_smart: [
        [
          0,
          {
            registered_at: 0,
            volume: "0",
            commission_earned: "0",
            last_day_active: 0,
          },
        ],
      ],
    });

    await expect(queryRefereeStats(client, perpsAddress, 0, orderBy)).resolves.toEqual([
      {
        userIndex: 0,
        registeredAt: 0,
        volume: "0",
        commissionEarned: "0",
        lastDayActive: 0,
      },
    ]);

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referrer_to_referee_stats: {
              referrer: 0,
              order_by: {
                order: "Ascending",
                limit: 25,
                index: {
                  volume: {
                    start_after: "0",
                  },
                },
              },
            },
          },
        },
      },
    });
  });

  it("serializes registration-time referee stats ordering with backend snake-case keys", async () => {
    const orderBy = {
      order: "Descending" as const,
      limit: 10,
      index: { registerAt: { startAfter: 1710000000 } },
    };
    const { client, queryApp } = createClient({
      wasm_smart: [],
    });

    await expect(queryRefereeStats(client, perpsAddress, 7, orderBy)).resolves.toEqual([]);

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referrer_to_referee_stats: {
              referrer: 7,
              order_by: {
                order: "Descending",
                limit: 10,
                index: {
                  register_at: {
                    start_after: 1710000000,
                  },
                },
              },
            },
          },
        },
      },
    });
  });

  it("serializes commission referee stats ordering and deserializes commission rows", async () => {
    const orderBy = {
      order: "Ascending" as const,
      limit: 5,
      index: { commission: { startAfter: "12.5" } },
    };
    const { client, queryApp } = createClient({
      wasm_smart: [
        [
          4,
          {
            registered_at: 10,
            volume: "250",
            commission_earned: "12.5",
            last_day_active: 11,
          },
        ],
      ],
    });

    await expect(queryRefereeStats(client, perpsAddress, 8, orderBy)).resolves.toEqual([
      {
        userIndex: 4,
        registeredAt: 10,
        volume: "250",
        commissionEarned: "12.5",
        lastDayActive: 11,
      },
    ]);

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referrer_to_referee_stats: {
              referrer: 8,
              order_by: {
                order: "Ascending",
                limit: 5,
                index: {
                  commission: {
                    start_after: "12.5",
                  },
                },
              },
            },
          },
        },
      },
    });
  });

  it("deserializes referrer settings and commission overrides", async () => {
    const settings = createClient({
      wasm_smart: {
        commission_rate: "0.25",
        share_ratio: "0.40",
      },
    });
    const override = createClient({ wasm_smart: "0.18" });

    await expect(queryReferralSettings(settings.client, perpsAddress, 9)).resolves.toEqual({
      commissionRate: "0.25",
      shareRatio: "0.40",
    });
    await expect(queryCommissionRateOverride(override.client, perpsAddress, 9)).resolves.toBe(
      "0.18",
    );
  });

  it("preserves zero-valued users in referral settings and commission override lookups", async () => {
    const settings = createClient({
      wasm_smart: {
        commission_rate: "0",
        share_ratio: "0",
      },
    });
    const override = createClient({ wasm_smart: "0" });

    await expect(queryReferralSettings(settings.client, perpsAddress, 0)).resolves.toEqual({
      commissionRate: "0",
      shareRatio: "0",
    });
    await expect(queryCommissionRateOverride(override.client, perpsAddress, 0)).resolves.toBe(
      "0",
    );

    expect(settings.queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referral_settings: {
              user: 0,
            },
          },
        },
      },
    });
    expect(override.queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            commission_rate_override: {
              user: 0,
            },
          },
        },
      },
    });
  });

  it("serializes referral settings lookups and preserves missing settings as null", async () => {
    const { client, queryApp } = createClient({ wasm_smart: null });

    await expect(queryReferralSettings(client, perpsAddress, 9)).resolves.toBeNull();

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            referral_settings: {
              user: 9,
            },
          },
        },
      },
    });
  });

  it("serializes commission override lookups and preserves missing overrides as null", async () => {
    const { client, queryApp } = createClient({ wasm_smart: null });

    await expect(queryCommissionRateOverride(client, perpsAddress, 9)).resolves.toBeNull();

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            commission_rate_override: {
              user: 9,
            },
          },
        },
      },
    });
  });

  it("extracts only referral-related params from the full perps param response", async () => {
    const { client, queryApp } = createClient({
      wasm_smart: {
        referral_active: true,
        min_referrer_volume: "10000",
        referrer_commission_rates: {
          base: "0.05",
          tiers: {
            "100000": "0.10",
          },
        },
        unrelated_param: "ignored",
      },
    });

    await expect(queryReferralParams(client, perpsAddress)).resolves.toEqual({
      referralActive: true,
      minReferrerVolume: "10000",
      referrerCommissionRates: {
        base: "0.05",
        tiers: {
          "100000": "0.10",
        },
      },
    });

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            param: {},
          },
        },
      },
    });
  });

  it("preserves disabled referral params and zero-valued commission thresholds", async () => {
    const { client, queryApp } = createClient({
      wasm_smart: {
        referral_active: false,
        min_referrer_volume: "0",
        referrer_commission_rates: {
          base: "0",
          tiers: {
            "0": "0",
          },
        },
      },
    });

    await expect(queryReferralParams(client, perpsAddress)).resolves.toEqual({
      referralActive: false,
      minReferrerVolume: "0",
      referrerCommissionRates: {
        base: "0",
        tiers: {
          "0": "0",
        },
      },
    });

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            param: {},
          },
        },
      },
    });
  });

  it("propagates public client query failures to the caller", async () => {
    const error = new Error("query failed");
    const { client, queryApp } = createRejectingClient(error);

    await expect(queryReferralParams(client, perpsAddress)).rejects.toBe(error);

    expect(queryApp).toHaveBeenCalledWith({
      query: {
        wasm_smart: {
          contract: perpsAddress,
          msg: {
            param: {},
          },
        },
      },
    });
  });
});
