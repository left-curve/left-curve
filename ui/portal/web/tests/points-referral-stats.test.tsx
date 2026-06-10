import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { setAppletsKitUseApp } from "./mocks/applets-kit";
import { Modals } from "@left-curve/applets-kit";

import { AffiliateStats, ReferralStats, TraderStats } from "../src/components/points/referral";

const referralStatsMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  setReferral: vi.fn(),
  showModal: vi.fn(),
  useAccount: vi.fn(),
  useCommissionRateOverride: vi.fn(),
  useReferralData: vi.fn(),
  useReferralParams: vi.fn(),
  useReferralSettings: vi.fn(),
  useReferrer: vi.fn(),
  useSetReferral: vi.fn(),
  useVolume: vi.fn(),
}));

class MockIntersectionObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

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
        dateFormat: "en-US",
        formatNumberOptions: {
          language: "en-US",
        },
      },
    }),
  };
});

vi.mock("@left-curve/store", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/store")>();

  return {
    ...actual,
    useAccount: referralStatsMocks.useAccount,
    useCommissionRateOverride: referralStatsMocks.useCommissionRateOverride,
    useReferralData: referralStatsMocks.useReferralData,
    useReferralParams: referralStatsMocks.useReferralParams,
    useReferralSettings: referralStatsMocks.useReferralSettings,
    useReferrer: referralStatsMocks.useReferrer,
    useSetReferral: referralStatsMocks.useSetReferral,
    useVolume: referralStatsMocks.useVolume,
  };
});

const accountAddress = "0x616666696c696174650000000000000000000000";

function referralData({
  activeReferees = 2,
  commission = "4321",
  refereeCount = 3,
  shared = "55",
  volume = "10000",
  refereesVolume = "900000",
}: Partial<{
  activeReferees: number;
  commission: string;
  refereeCount: number;
  refereesVolume: string;
  shared: string;
  volume: string;
}> = {}) {
  return {
    commissionEarnedFromReferees: commission,
    commissionSharedByReferrer: shared,
    cumulativeDailyActiveReferees: activeReferees,
    cumulativeGlobalActiveReferees: activeReferees,
    refereeCount,
    refereesVolume,
    volume,
  };
}

function setConnectedAccount({
  isConnected = true,
  userIndex = 42,
}: Partial<{
  isConnected: boolean;
  userIndex: number | undefined;
}> = {}) {
  referralStatsMocks.useAccount.mockReturnValue({
    account: isConnected
      ? {
          address: accountAddress,
        }
      : undefined,
    isConnected,
    userIndex,
  });
}

function setReferralParams() {
  referralStatsMocks.useReferralParams.mockReturnValue({
    isLoading: false,
    referralParams: {
      minReferrerVolume: "10000",
      referralActive: true,
      referrerCommissionRates: {
        base: "0.05",
        tiers: {
          "100000": "0.10",
          "250000": "0.15",
        },
      },
    },
  });
}

function setAffiliateData({
  settings = {
    commissionRate: "0.10",
    shareRatio: "0.40",
  },
  totalData = referralData(),
  volume = "50000",
  windowData = referralData({
    activeReferees: 1,
    commission: "0",
    refereeCount: 1,
    refereesVolume: "120000",
  }),
}: Partial<{
  settings: null | {
    commissionRate: string;
    shareRatio: string;
  };
  totalData: ReturnType<typeof referralData>;
  volume: string;
  windowData: ReturnType<typeof referralData>;
}> = {}) {
  referralStatsMocks.useReferralSettings.mockReturnValue({
    isLoading: false,
    settings,
  });
  referralStatsMocks.useReferralData.mockImplementation(({ since }: { since?: number } = {}) => ({
    isLoading: false,
    referralData: since == null ? totalData : windowData,
  }));
  referralStatsMocks.useVolume.mockReturnValue({
    isLoading: false,
    volume,
  });
  referralStatsMocks.useCommissionRateOverride.mockReturnValue({
    hasOverride: false,
    isLoading: false,
    override: null,
  });
}

function setTraderData({
  hasReferrer = false,
  referrer = null,
  settings = {
    commissionRate: "0.10",
    shareRatio: "0.25",
  },
  totalData = referralData(),
}: Partial<{
  hasReferrer: boolean;
  referrer: number | null;
  settings: null | {
    commissionRate: string;
    shareRatio: string;
  };
  totalData: ReturnType<typeof referralData>;
}> = {}) {
  referralStatsMocks.useReferrer.mockReturnValue({
    hasReferrer,
    isLoading: false,
    referrer,
  });
  referralStatsMocks.useReferralData.mockReturnValue({
    isLoading: false,
    referralData: totalData,
  });
  referralStatsMocks.useReferralSettings.mockReturnValue({
    isLoading: false,
    settings,
  });
}

describe("points referral stats", () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, "IntersectionObserver", {
      configurable: true,
      value: MockIntersectionObserver,
    });
    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: MockResizeObserver,
    });
    setAppletsKitUseApp({
      navigate: referralStatsMocks.navigate,
      settings: {
        dateFormat: "en-US",
        formatNumberOptions: {
          language: "en-US",
        },
      },
      showModal: referralStatsMocks.showModal,
    });

    setConnectedAccount();
    setReferralParams();
    setAffiliateData();
    setTraderData();
    referralStatsMocks.useSetReferral.mockImplementation(
      ({ onSuccess }: { onSuccess?: () => void } = {}) => ({
        isPending: false,
        mutate: (variables: { referee: number; referrer: number }) => {
          referralStatsMocks.setReferral(variables);
          onSuccess?.();
        },
      }),
    );
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders affiliate commission splits, tier progress, and referrer credentials from backend data", () => {
    setAffiliateData();

    render(<AffiliateStats />);

    expect(referralStatsMocks.useVolume).toHaveBeenCalledWith({
      enabled: true,
      since: undefined,
      userAddress: accountAddress,
    });
    expect(referralStatsMocks.useReferralData).toHaveBeenCalledWith({
      userIndex: 42,
    });
    expect(referralStatsMocks.useReferralData).toHaveBeenCalledWith(
      expect.objectContaining({
        enabled: true,
        userIndex: 42,
      }),
    );

    expect(screen.getByText("6% / 4%")).toBeInTheDocument();
    expect(screen.getByText(m["referral.stats.commissionRate"]())).toBeInTheDocument();
    expect(document.body).toHaveTextContent(/\$4,321(\.00)?/);
    expect(document.body).toHaveTextContent(/\$900,000(\.00)?/);
    expect(document.body).toHaveTextContent("$130,000");
    expect(document.body).toHaveTextContent("Tier 3");
    expect(screen.getByText("$250K")).toBeInTheDocument();

    expect(screen.getByText(m["referral.stats.totalReferees"]()).parentElement).toHaveTextContent(
      "3",
    );
    expect(
      screen.getByText(m["referral.stats.totalActiveReferees"]()).parentElement,
    ).toHaveTextContent("2");
    expect(screen.getByText(m["referral.stats.myReferralCode"]()).parentElement).toHaveTextContent(
      "42",
    );
  });

  it("renders backend zero affiliate totals as connected account metrics", () => {
    setAffiliateData({
      settings: {
        commissionRate: "0",
        shareRatio: "0",
      },
      totalData: referralData({
        activeReferees: 0,
        commission: "0",
        refereeCount: 0,
        refereesVolume: "0",
        shared: "0",
        volume: "0",
      }),
      volume: "0",
      windowData: referralData({
        activeReferees: 0,
        commission: "0",
        refereeCount: 0,
        refereesVolume: "0",
        shared: "0",
        volume: "0",
      }),
    });

    render(<AffiliateStats />);

    expect(screen.getByText("0% / 0%")).toBeInTheDocument();
    expect(screen.getByText(m["referral.stats.commissionRate"]())).toBeInTheDocument();
    expect(document.body.textContent?.match(/\$0(?:\.00)?/g)).toHaveLength(2);
    expect(screen.getByText(m["referral.stats.totalReferees"]()).parentElement).toHaveTextContent(
      "0",
    );
    expect(
      screen.getByText(m["referral.stats.totalActiveReferees"]()).parentElement,
    ).toHaveTextContent("0");
    expect(screen.getByText(m["referral.stats.myReferralCode"]()).parentElement).toHaveTextContent(
      "42",
    );
    expect(
      screen.queryByRole("button", { name: m["referral.affiliateSection.tradeNow"]() }),
    ).not.toBeInTheDocument();
  });

  it("routes affiliate locked states to authentication or fee-share setup", () => {
    setConnectedAccount({
      isConnected: false,
      userIndex: undefined,
    });

    const { rerender } = render(<AffiliateStats />);

    fireEvent.click(screen.getByRole("button", { name: m["referral.affiliateSection.logIn"]() }));

    expect(referralStatsMocks.showModal).toHaveBeenCalledWith(Modals.Authenticate, {
      action: "signin",
    });

    setConnectedAccount();
    setAffiliateData({
      settings: null,
      volume: "10000",
    });

    rerender(<AffiliateStats />);

    expect(screen.getByText(m["referral.affiliateSection.setFeeShareTitle"]())).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: m["referral.affiliateSection.setFeeShareRate"]() }),
    );

    expect(referralStatsMocks.showModal).toHaveBeenCalledWith(Modals.EditCommissionRate);
  });

  it("lets backend commission overrides unlock affiliate fee-share setup below volume threshold", () => {
    setAffiliateData({
      settings: null,
      volume: "0",
      windowData: referralData({
        activeReferees: 0,
        commission: "0",
        refereeCount: 0,
        refereesVolume: "0",
        shared: "0",
        volume: "0",
      }),
    });
    referralStatsMocks.useCommissionRateOverride.mockReturnValue({
      hasOverride: true,
      isLoading: false,
      override: "0.18",
    });

    render(<AffiliateStats />);

    expect(referralStatsMocks.useCommissionRateOverride).toHaveBeenCalledWith({
      enabled: true,
      userIndex: 42,
    });
    expect(screen.getByText("18% / 0%")).toBeInTheDocument();
    expect(screen.getByText(m["referral.affiliateSection.setFeeShareTitle"]())).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: m["referral.affiliateSection.tradeNow"]() }),
    ).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: m["referral.affiliateSection.setFeeShareRate"]() }),
    );

    expect(referralStatsMocks.showModal).toHaveBeenCalledWith(Modals.EditCommissionRate);
  });

  it("submits trader referral codes as numeric backend identifiers and clears the input", () => {
    render(<TraderStats />);

    expect(screen.getByText(m["referral.traderSection.referralCodeLabel"]())).toBeInTheDocument();
    const codeInput = screen.getByRole("textbox");

    fireEvent.change(codeInput, {
      target: {
        value: "17",
      },
    });
    fireEvent.click(screen.getByRole("button", { name: m["referral.traderSection.submit"]() }));

    expect(referralStatsMocks.setReferral).toHaveBeenCalledWith({
      referee: 42,
      referrer: 17,
    });
    expect(codeInput).toHaveValue("");
  });

  it("renders trader rebate totals and the existing referrer badge when the backend has a referrer", () => {
    setTraderData({
      hasReferrer: true,
      referrer: 13,
      totalData: referralData({
        shared: "55",
        volume: "10000",
      }),
    });

    render(<TraderStats />);

    expect(referralStatsMocks.useReferralSettings).toHaveBeenCalledWith({
      enabled: true,
      userIndex: 13,
    });
    expect(screen.getByText("25%")).toBeInTheDocument();
    expect(screen.getByText(m["referral.stats.totalRebates"]()).parentElement).toHaveTextContent(
      /\$55(\.00)?/,
    );
    expect(
      screen.getByText(m["referral.stats.totalTradingVolume"]()).parentElement,
    ).toHaveTextContent("$10,000");
    expect(screen.getByText(m["referral.stats.yourReferrer"]())).toBeInTheDocument();
    expect(screen.getByText("#13")).toBeInTheDocument();
  });

  it("renders backend zero trader rebate totals while preserving the existing referrer", () => {
    setTraderData({
      hasReferrer: true,
      referrer: 13,
      settings: {
        commissionRate: "0",
        shareRatio: "0",
      },
      totalData: referralData({
        shared: "0",
        volume: "0",
      }),
    });

    render(<TraderStats />);

    expect(referralStatsMocks.useReferralSettings).toHaveBeenCalledWith({
      enabled: true,
      userIndex: 13,
    });
    expect(screen.getByText("0%")).toBeInTheDocument();
    expect(screen.getByText(m["referral.stats.totalRebates"]()).parentElement).toHaveTextContent(
      /\$0(\.00)?/,
    );
    expect(
      screen.getByText(m["referral.stats.totalTradingVolume"]()).parentElement,
    ).toHaveTextContent(/\$0(\.00)?/);
    expect(screen.getByText(m["referral.stats.yourReferrer"]())).toBeInTheDocument();
    expect(screen.getByText("#13")).toBeInTheDocument();
  });

  it("renders the referral mode tabs with the affiliate tier badge and forwards mode changes", () => {
    const onModeChange = vi.fn();

    render(<ReferralStats mode="affiliate" onModeChange={onModeChange} />);

    expect(screen.getByText(m["referral.affiliate"]())).toBeInTheDocument();
    expect(screen.getByText("Tier 2")).toBeInTheDocument();
    expect(screen.getByText(m["referral.trader"]())).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: m["referral.trader"]() }));

    expect(onModeChange).toHaveBeenCalledWith("trader");
  });
});
