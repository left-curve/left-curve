import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { ReferralCampaign } from "../src/components/referral/ReferralCampaign";

type ReferralMode = "affiliate" | "trader";

const referralCampaignMocks = vi.hoisted(() => ({
  open: vi.fn(),
}));

class MockResizeObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

vi.mock("../src/components/foundation/MobileTitle", () => ({
  MobileTitle: ({ title }: { title: string }) => <div data-testid="mobile-title">{title}</div>,
}));

vi.mock("../src/components/foundation/PageGlow", () => ({
  PageGlow: () => <div data-testid="page-glow" />,
}));

vi.mock("../src/components/points/referral", () => ({
  AffiliateStats: () => <section data-testid="affiliate-stats" />,
  CommissionRates: () => <section data-testid="commission-rates" />,
  MyCommission: ({ mode }: { mode: ReferralMode }) => (
    <section data-mode={mode} data-testid="my-commission" />
  ),
  ReferralFaqs: () => <section data-testid="referral-faqs" />,
  TraderStats: () => <section data-testid="trader-stats" />,
}));

function renderCampaign({
  activeTab = "affiliate",
  onTabChange = vi.fn(),
}: Partial<{
  activeTab: ReferralMode;
  onTabChange: (tab: ReferralMode) => void;
}> = {}) {
  render(
    <ReferralCampaign activeTab={activeTab} onTabChange={onTabChange}>
      <ReferralCampaign.Header />
      <ReferralCampaign.Content />
    </ReferralCampaign>,
  );

  return { onTabChange };
}

describe("ReferralCampaign", () => {
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", MockResizeObserver);
    vi.stubGlobal("open", referralCampaignMocks.open);
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("renders the affiliate view with commission rates, affiliate commission, and FAQs", () => {
    renderCampaign();

    expect(screen.getByTestId("mobile-title")).toHaveTextContent(m["referral.mobileTitle"]());
    expect(screen.getByText(m["referral.welcome"]())).toBeInTheDocument();
    expect(screen.getByText(m["referral.title"]())).toBeInTheDocument();
    expect(screen.getByTestId("affiliate-stats")).toBeInTheDocument();
    expect(screen.getByTestId("commission-rates")).toBeInTheDocument();
    expect(screen.getByTestId("my-commission")).toHaveAttribute("data-mode", "affiliate");
    expect(screen.getByTestId("referral-faqs")).toBeInTheDocument();
    expect(screen.queryByTestId("trader-stats")).not.toBeInTheDocument();
  });

  it("renders the trader view without affiliate-only content", () => {
    renderCampaign({
      activeTab: "trader",
    });

    expect(screen.getByTestId("trader-stats")).toBeInTheDocument();
    expect(screen.getByTestId("my-commission")).toHaveAttribute("data-mode", "trader");
    expect(screen.queryByTestId("affiliate-stats")).not.toBeInTheDocument();
    expect(screen.queryByTestId("commission-rates")).not.toBeInTheDocument();
    expect(screen.queryByTestId("referral-faqs")).not.toBeInTheDocument();
  });

  it("delegates tab changes through the controlled campaign contract", () => {
    const { onTabChange } = renderCampaign();

    fireEvent.click(screen.getByRole("button", { name: m["referral.trader"]() }));
    fireEvent.click(screen.getByRole("button", { name: `${m["referral.affiliate"]()} Tier 1` }));

    expect(onTabChange).toHaveBeenNthCalledWith(1, "trader");
    expect(onTabChange).toHaveBeenNthCalledWith(2, "affiliate");
  });

  it("opens the referral rules from the header", () => {
    renderCampaign();

    fireEvent.click(screen.getByRole("button", { name: m["referral.readRules"]() }));

    expect(referralCampaignMocks.open).toHaveBeenCalledWith(
      "https://dango-4.gitbook.io/dango-docs/referral-system",
    );
  });
});
