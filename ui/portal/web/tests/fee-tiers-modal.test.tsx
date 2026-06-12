import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";

import { FeeTiers } from "../src/components/modals/FeeTiers";

const feeTierModalMocks = vi.hoisted(() => ({
  hideModal: vi.fn(),
  useAppConfig: vi.fn(),
  useFeeRateOverride: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useAppConfig: feeTierModalMocks.useAppConfig,
  useFeeRateOverride: feeTierModalMocks.useFeeRateOverride,
}));

function getRows() {
  return screen.getAllByRole("row").slice(1);
}

function rowText(row: HTMLElement) {
  return within(row)
    .getAllByRole("cell")
    .map((cell) => cell.textContent);
}

function getCloseButton(container: HTMLElement) {
  const button = container.querySelector("button.absolute");
  if (!(button instanceof HTMLButtonElement)) throw new Error("Could not find close button");
  return button;
}

describe("fee tiers modal", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: feeTierModalMocks.hideModal,
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
    });
    feeTierModalMocks.useAppConfig.mockReturnValue({
      data: {
        perpsParam: {
          makerFeeRates: {
            base: "-0.0001",
            tiers: {
              "250000": "-0.0002",
              "500000": "-0.0003",
            },
          },
          takerFeeRates: {
            base: "0.001",
            tiers: {
              "100000": "0.0008",
              "500000": "0.0005",
            },
          },
        },
      },
    });
    feeTierModalMocks.useFeeRateOverride.mockReturnValue({
      override: undefined,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("merges taker and maker fee schedules by sorted 14-day volume thresholds", () => {
    render(<FeeTiers />);

    expect(screen.getByText(m["dex.feeTiers.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.feeTiers.description"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.feeTiers.tier"]())).toBeVisible();
    expect(screen.getByText(m["dex.feeTiers.volume14d"]())).toBeVisible();
    expect(screen.getByText(m["dex.feeTiers.perpsTaker"]())).toBeVisible();
    expect(screen.getByText(m["dex.feeTiers.perpsMaker"]())).toBeVisible();

    expect(getRows().map(rowText)).toEqual([
      ["0", "--", "0.1%", "-0.01%"],
      ["1", "$100,000", "0.08%", "-0.01%"],
      ["2", "$250,000", "0.08%", "-0.02%"],
      ["3", "$500,000", "0.05%", "-0.03%"],
    ]);
  });

  it("shows custom fee-rate overrides and closes from the icon button", () => {
    feeTierModalMocks.useFeeRateOverride.mockReturnValue({
      override: {
        makerFeeRate: "-0.0004",
        takerFeeRate: "0.0007",
      },
    });

    const { container } = render(<FeeTiers />);

    expect(screen.getByText(m["dex.feeTiers.customRate"]())).toBeInTheDocument();
    expect(screen.getByText(m["dex.feeTiers.customRateDescription"]())).toBeInTheDocument();
    expect(screen.getByText(`${m["dex.feeTiers.perpsTaker"]()}: 0.07%`)).toBeInTheDocument();
    expect(screen.getByText(`${m["dex.feeTiers.perpsMaker"]()}: -0.04%`)).toBeInTheDocument();

    fireEvent.click(getCloseButton(container));

    expect(feeTierModalMocks.hideModal).toHaveBeenCalledOnce();
  });

  it("renders an empty table when app config has no fee schedules yet", () => {
    feeTierModalMocks.useAppConfig.mockReturnValue({
      data: {
        perpsParam: undefined,
      },
    });

    render(<FeeTiers />);

    expect(screen.getAllByRole("row")).toHaveLength(1);
  });
});
