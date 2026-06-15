import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks } from "./mocks/applets-kit";

import { CommissionRates } from "../src/components/points/referral";

const commissionRatesMocks = vi.hoisted(() => ({
  useAccount: vi.fn(),
  useReferralParams: vi.fn(),
}));

class MockResizeObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

vi.mock("@left-curve/store", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/store")>();

  return {
    ...actual,
    useAccount: commissionRatesMocks.useAccount,
    useReferralParams: commissionRatesMocks.useReferralParams,
  };
});

function setAccount({ isConnected = true }: { isConnected?: boolean } = {}) {
  commissionRatesMocks.useAccount.mockReturnValue({
    isConnected,
  });
}

function setReferralParams({
  isLoading = false,
  referralParams = {
    minReferrerVolume: "10000",
    referralActive: true,
    referrerCommissionRates: {
      base: "0.05",
      tiers: {
        "250000": "0.15",
        "100000": "0.10",
      },
    },
  },
}: Partial<{
  isLoading: boolean;
  referralParams: null | {
    minReferrerVolume: string;
    referralActive: boolean;
    referrerCommissionRates: {
      base: string;
      tiers: Record<string, string>;
    };
  };
}> = {}) {
  commissionRatesMocks.useReferralParams.mockReturnValue({
    isLoading,
    referralParams,
  });
}

function bodyRows() {
  return screen.getAllByRole("row").slice(1);
}

function rowText(row: HTMLElement) {
  return row.textContent ?? "";
}

describe("CommissionRates", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: MockResizeObserver,
    });

    setAccount();
    setReferralParams();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders connected commission tiers from referral params in numeric threshold order", () => {
    render(<CommissionRates />);

    expect(screen.getByText(m["referral.commission.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["referral.commission.columns.tier"]())).toBeInTheDocument();
    expect(screen.getByText(m["referral.commission.columns.tradingVolume"]())).toBeInTheDocument();
    expect(
      screen.getByText(m["referral.commission.columns.thirtyDayReferralVolume"]()),
    ).toBeInTheDocument();
    expect(screen.getByText(m["referral.commission.columns.commission"]())).toBeInTheDocument();

    const rows = bodyRows();
    expect(rows).toHaveLength(3);
    expect(rowText(rows[0])).toContain("Tier 1");
    expect(rowText(rows[0])).toMatch(/\$10,000(?:\.00)?/);
    expect(rowText(rows[0])).toContain("0");
    expect(rowText(rows[0])).toContain("5%");

    expect(rowText(rows[1])).toContain("Tier 2");
    expect(rowText(rows[1])).toMatch(/\$100,000(?:\.00)?/);
    expect(rowText(rows[1])).toContain("10%");

    expect(rowText(rows[2])).toContain("Tier 3");
    expect(rowText(rows[2])).toMatch(/\$250,000(?:\.00)?/);
    expect(rowText(rows[2])).toContain("15%");
  });

  it("renders only tier one when the backend has no configured volume tiers", () => {
    setReferralParams({
      referralParams: {
        minReferrerVolume: "50000",
        referralActive: true,
        referrerCommissionRates: {
          base: "0.07",
          tiers: {},
        },
      },
    });

    render(<CommissionRates />);

    const rows = bodyRows();
    expect(rows).toHaveLength(1);
    expect(rowText(rows[0])).toContain("Tier 1");
    expect(rowText(rows[0])).toMatch(/\$50,000(?:\.00)?/);
    expect(rowText(rows[0])).toContain("7%");
  });

  it("renders backend zero referral thresholds and commission rates as real tiers", () => {
    setReferralParams({
      referralParams: {
        minReferrerVolume: "0",
        referralActive: true,
        referrerCommissionRates: {
          base: "0",
          tiers: {
            "0": "0",
          },
        },
      },
    });

    render(<CommissionRates />);

    const rows = bodyRows();
    expect(rows).toHaveLength(2);
    expect(rowText(rows[0])).toContain("Tier 1");
    expect(rowText(rows[0])).toMatch(/\$0(?:\.00)?/);
    expect(rowText(rows[0])).toContain("0%");

    expect(rowText(rows[1])).toContain("Tier 2");
    expect(rowText(rows[1])).toMatch(/\$0(?:\.00)?/);
    expect(rowText(rows[1])).toContain("0%");
  });

  it("does not render commission rates for disconnected users or missing params", () => {
    setAccount({
      isConnected: false,
    });

    const { rerender } = render(<CommissionRates />);

    expect(screen.queryByText(m["referral.commission.title"]())).not.toBeInTheDocument();

    setAccount();
    setReferralParams({
      referralParams: null,
    });

    rerender(<CommissionRates />);

    expect(screen.queryByText(m["referral.commission.title"]())).not.toBeInTheDocument();
  });
});
