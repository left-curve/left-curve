import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { FundingCountdown } from "../src/components/dex/components/FundingCountdown";
import { OpenInterestDisplay } from "../src/components/dex/components/OpenInterestDisplay";

const perpsIndicatorMocks = vi.hoisted(() => ({
  currentPrice: "100",
  fundingPeriod: 3600,
  lastFundingTime: "1717243200",
  pairParam: {
    maxAbsOi: "5",
  },
  pairState: {
    fundingRate: "-0.00024",
    longOi: "1",
    shortOi: "2",
  },
}));

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      settings: {
        formatNumberOptions: {
          language: "en-US",
          mask: 1,
        },
      },
    }),
    useCountdown: ({ date }: { date?: number }) =>
      date
        ? {
            hours: "01",
            minutes: "02",
            seconds: "03",
          }
        : {
            hours: "-",
            minutes: "-",
            seconds: "-",
          },
  };
});

vi.mock("../src/components/dex/components/ProTrade", () => ({
  useProTrade: () => ({
    perpsPairId: "perp/btcusd",
  }),
}));

vi.mock("@left-curve/store", () => ({
  useCurrentPrice: () => ({
    currentPrice: perpsIndicatorMocks.currentPrice,
  }),
  usePerpsPairParam: () => ({
    data: perpsIndicatorMocks.pairParam,
  }),
  usePerpsPairState: (
    selector: (state: { pairState?: typeof perpsIndicatorMocks.pairState }) => unknown,
  ) =>
    selector({
      pairState: perpsIndicatorMocks.pairState,
    }),
  usePerpsParam: () => ({
    data:
      perpsIndicatorMocks.fundingPeriod === undefined
        ? undefined
        : {
            fundingPeriod: perpsIndicatorMocks.fundingPeriod,
          },
  }),
  usePerpsState: (selector: (state: { state?: { lastFundingTime?: string } }) => unknown) =>
    selector({
      state:
        perpsIndicatorMocks.lastFundingTime === undefined
          ? undefined
          : {
              lastFundingTime: perpsIndicatorMocks.lastFundingTime,
            },
    }),
}));

function textElement(text: string, tagName: string) {
  return screen.getByText((_, node) => node?.tagName === tagName && node.textContent === text);
}

describe("perps risk and funding indicators", () => {
  beforeEach(() => {
    perpsIndicatorMocks.currentPrice = "100";
    perpsIndicatorMocks.fundingPeriod = 3600;
    perpsIndicatorMocks.lastFundingTime = "1717243200";
    perpsIndicatorMocks.pairParam = {
      maxAbsOi: "5",
    };
    perpsIndicatorMocks.pairState = {
      fundingRate: "-0.00024",
      longOi: "1",
      shortOi: "2",
    };
  });

  afterEach(() => {
    cleanup();
  });

  it("renders negative hourly funding in the loss style with the next funding countdown", () => {
    render(<FundingCountdown />);

    expect(screen.getByText(m["dex.protrade.spot.funding"]())).toBeInTheDocument();
    expect(textElement("-0.001%", "SPAN")).toHaveClass("text-status-fail");
    expect(screen.getByText("01:02:03")).toBeInTheDocument();
  });

  it("falls back to zero funding and an empty countdown when funding schedule data is missing", () => {
    perpsIndicatorMocks.pairState = {
      ...perpsIndicatorMocks.pairState,
      fundingRate: "",
    };
    perpsIndicatorMocks.lastFundingTime = undefined;
    perpsIndicatorMocks.fundingPeriod = undefined;

    render(<FundingCountdown />);

    expect(screen.getByText("0.00%")).toBeInTheDocument();
    expect(screen.getByText("-:-:-")).toBeInTheDocument();
  });

  it("renders total open interest without the limit warning below pair limits", () => {
    const { container } = render(<OpenInterestDisplay />);

    expect(screen.getByText(m["dex.protrade.spot.openInterest"]())).toBeInTheDocument();
    expect(textElement("$300.00", "P")).toBeInTheDocument();
    expect(container.querySelector("svg.text-status-fail")).toBeNull();
  });

  it("renders backend zero open interest values instead of the missing-data placeholder", () => {
    perpsIndicatorMocks.currentPrice = "0";
    perpsIndicatorMocks.pairState = {
      ...perpsIndicatorMocks.pairState,
      longOi: "0",
      shortOi: "0",
    };

    const { container } = render(<OpenInterestDisplay />);

    expect(textElement("$0.00", "P")).toBeInTheDocument();
    expect(screen.queryByText("-")).not.toBeInTheDocument();
    expect(container.querySelector("svg.text-status-fail")).toBeNull();
  });

  it("shows the open-interest warning style when either side reaches the pair limit", () => {
    perpsIndicatorMocks.pairState = {
      ...perpsIndicatorMocks.pairState,
      longOi: "5",
    };

    const { container } = render(<OpenInterestDisplay />);

    expect(textElement("$700.00", "P")).toHaveClass("text-status-fail");
    expect(container.querySelector("svg.text-status-fail")).not.toBeNull();
  });
});
