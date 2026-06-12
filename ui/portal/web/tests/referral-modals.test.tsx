import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { EditCommissionRate } from "../src/components/modals/EditCommissionRate";

const referralModalMocks = vi.hoisted(() => ({
  hideModal: vi.fn(),
  setFeeShareRatio: vi.fn(),
  useCommissionRateOverride: vi.fn(),
  useReferralParams: vi.fn(),
  useReferralSettings: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: () => ({
    userIndex: 42,
  }),
  useCommissionRateOverride: referralModalMocks.useCommissionRateOverride,
  useReferralParams: referralModalMocks.useReferralParams,
  useReferralSettings: referralModalMocks.useReferralSettings,
  useSetFeeShareRatio: ({ onSuccess }: { onSuccess?: () => void } = {}) => ({
    isPending: false,
    mutate: (variables: { shareRatio: string }) => {
      try {
        referralModalMocks.setFeeShareRatio(variables);
        onSuccess?.();
      } catch {
        // React Query's mutate path reports failures through mutation state/callbacks.
      }
    },
  }),
}));

function setReferralState({
  commissionRate = "0.20",
  isLoading = false,
  override,
  shareRatio = "0.25",
}: {
  commissionRate?: string;
  isLoading?: boolean;
  override?: string;
  shareRatio?: string;
} = {}) {
  referralModalMocks.useReferralSettings.mockReturnValue({
    isLoading,
    settings: isLoading ? undefined : { commissionRate, shareRatio },
  });
  referralModalMocks.useCommissionRateOverride.mockReturnValue({
    isLoading,
    override,
  });
  referralModalMocks.useReferralParams.mockReturnValue({
    referralParams: {
      referrerCommissionRates: {
        base: "0.05",
        tiers: {},
      },
    },
  });
}

function commissionInputs() {
  const [youReceiveInput, refereeReceivesInput] = screen.getAllByRole("textbox");
  return { refereeReceivesInput, youReceiveInput };
}

describe("referral modals", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: referralModalMocks.hideModal,
    });
    setReferralState();
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("saves an increased referee fee share as the backend share ratio", () => {
    render(<EditCommissionRate />);

    const { refereeReceivesInput, youReceiveInput } = commissionInputs();

    expect(screen.getByText("20%")).toBeInTheDocument();
    expect(youReceiveInput).toHaveValue("15");
    expect(refereeReceivesInput).toHaveValue("5");

    fireEvent.change(refereeReceivesInput, {
      target: { value: "8" },
    });

    expect(youReceiveInput).toHaveValue("12");
    fireEvent.click(screen.getByRole("button", { name: m["referral.editFeeShare.save"]() }));

    expect(referralModalMocks.setFeeShareRatio).toHaveBeenCalledWith({
      shareRatio: "0.4",
    });
    expect(referralModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("uses commission overrides when deriving the displayed split and backend share ratio", () => {
    setReferralState({
      commissionRate: "0.20",
      override: "0.30",
      shareRatio: "0.25",
    });

    render(<EditCommissionRate />);

    const { refereeReceivesInput, youReceiveInput } = commissionInputs();

    expect(screen.getByText("30%")).toBeInTheDocument();
    expect(youReceiveInput).toHaveValue("22.5");
    expect(refereeReceivesInput).toHaveValue("7.5");

    fireEvent.change(refereeReceivesInput, {
      target: { value: "12" },
    });

    expect(youReceiveInput).toHaveValue("18");
    fireEvent.click(screen.getByRole("button", { name: m["referral.editFeeShare.save"]() }));

    expect(referralModalMocks.setFeeShareRatio).toHaveBeenCalledWith({
      shareRatio: "0.4",
    });
    expect(referralModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("keeps the modal open when backend fee-share updates fail", () => {
    referralModalMocks.setFeeShareRatio.mockImplementationOnce(() => {
      throw new Error("fee share rejected");
    });
    render(<EditCommissionRate />);

    fireEvent.change(commissionInputs().refereeReceivesInput, {
      target: { value: "8" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["referral.editFeeShare.save"]() }));

    expect(referralModalMocks.setFeeShareRatio).toHaveBeenCalledWith({
      shareRatio: "0.4",
    });
    expect(referralModalMocks.hideModal).not.toHaveBeenCalled();
  });

  it("blocks decreasing the current referee share", () => {
    render(<EditCommissionRate />);

    fireEvent.change(commissionInputs().refereeReceivesInput, {
      target: { value: "4" },
    });

    expect(screen.getByText(m["referral.editFeeShare.errorDecrease"]())).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: m["referral.editFeeShare.save"]() }));
    expect(referralModalMocks.setFeeShareRatio).not.toHaveBeenCalled();
  });

  it("blocks referee shares above the protocol maximum", () => {
    render(<EditCommissionRate />);

    fireEvent.change(commissionInputs().refereeReceivesInput, {
      target: { value: "11" },
    });

    expect(screen.getByText(m["referral.editFeeShare.errorExceedsMax"]())).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: m["referral.editFeeShare.save"]() }));
    expect(referralModalMocks.setFeeShareRatio).not.toHaveBeenCalled();
  });

  it("blocks referee shares above the total commission rate before submission", () => {
    setReferralState({
      commissionRate: "0.05",
      shareRatio: "0",
    });

    render(<EditCommissionRate />);

    fireEvent.change(commissionInputs().refereeReceivesInput, {
      target: { value: "6" },
    });

    expect(
      screen.getByText(m["referral.editFeeShare.errorExceedsCommission"]()),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: m["referral.editFeeShare.save"]() }));
    expect(referralModalMocks.setFeeShareRatio).not.toHaveBeenCalled();
  });

  it("uses referral params while loading user-specific commission settings", () => {
    setReferralState({ isLoading: true });

    const { container } = render(<EditCommissionRate />);

    expect(container.querySelectorAll(".animate-pulse")).toHaveLength(2);
    expect(screen.getByRole("button", { name: m["referral.editFeeShare.save"]() })).toBeVisible();
    expect(referralModalMocks.setFeeShareRatio).not.toHaveBeenCalled();
  });
});
