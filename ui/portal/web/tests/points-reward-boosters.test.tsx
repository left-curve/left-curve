import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "./mocks/applets-kit";
import { BoostersSection } from "../src/components/points/rewards/BoostersSection";

function huntedBooster({
  epoch,
  loot,
  multiplier,
}: {
  epoch: number;
  loot: "bronze_shell" | "golden_shell" | "pearl_dango" | "silver_shell";
  multiplier: string;
}) {
  return {
    epoch,
    loot,
    multiplier: {
      toString: () => multiplier,
    },
    rank: 0,
  };
}

describe("BoostersSection", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      settings: {
        formatNumberOptions: {
          language: "en-US",
        },
        timeZone: "UTC",
      },
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders current epoch boosters with localized expiration and locked fallback tiers", () => {
    render(
      <BoostersSection
        currentEpoch={9}
        currentEpochEndsAt={new Date("2026-06-09T15:30:00Z")}
        huntedBoosters={[
          huntedBooster({
            epoch: 9,
            loot: "silver_shell",
            multiplier: "1.75",
          }),
          huntedBooster({
            epoch: 9,
            loot: "golden_shell",
            multiplier: "2.25",
          }),
          huntedBooster({
            epoch: 8,
            loot: "pearl_dango",
            multiplier: "3",
          }),
        ]}
      />,
    );

    expect(screen.getByText(m["points.boosters.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["points.boosters.description"]())).toBeInTheDocument();

    expect(
      screen.getByAltText(m["points.boosters.multiplierLabel"]({ multiplier: "1.25" })),
    ).toBeInTheDocument();
    expect(
      screen.getByText(m["points.boosters.multiplierLabel"]({ multiplier: "1.75" })),
    ).toBeInTheDocument();
    expect(
      screen.getByText(m["points.boosters.multiplierLabel"]({ multiplier: "2.25" })),
    ).toBeInTheDocument();
    expect(
      screen.getByText(m["points.boosters.multiplierLabel"]({ multiplier: "2.5" })),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(m["points.boosters.multiplierLabel"]({ multiplier: "3" })),
    ).not.toBeInTheDocument();

    expect(screen.getAllByText("Jun 9, 15:30")).toHaveLength(2);
    expect(screen.getAllByText(m["points.boosters.locked"]())).toHaveLength(2);
  });

  it("shows owned multipliers but locked expirations when the epoch end date is missing", () => {
    render(
      <BoostersSection
        currentEpoch={9}
        currentEpochEndsAt={null}
        huntedBoosters={[
          huntedBooster({
            epoch: 9,
            loot: "bronze_shell",
            multiplier: "9",
          }),
        ]}
      />,
    );

    expect(
      screen.getByText(m["points.boosters.multiplierLabel"]({ multiplier: "9" })),
    ).toBeInTheDocument();
    expect(screen.getAllByText(m["points.boosters.locked"]())).toHaveLength(4);
    for (const multiplier of ["1.5", "2", "2.5"]) {
      expect(
        screen.getByText(m["points.boosters.multiplierLabel"]({ multiplier })),
      ).toBeInTheDocument();
    }
    expect(
      screen.queryByText(m["points.boosters.multiplierLabel"]({ multiplier: "1.25" })),
    ).not.toBeInTheDocument();
  });

  it("renders backend identity multipliers as owned boosters instead of locked fallbacks", () => {
    render(
      <BoostersSection
        currentEpoch={9}
        currentEpochEndsAt={new Date("2026-06-09T15:30:00Z")}
        huntedBoosters={[
          huntedBooster({
            epoch: 9,
            loot: "bronze_shell",
            multiplier: "1",
          }),
        ]}
      />,
    );

    expect(
      screen.getByText(m["points.boosters.multiplierLabel"]({ multiplier: "1" })),
    ).toBeInTheDocument();
    expect(
      screen.getByAltText(m["points.boosters.multiplierLabel"]({ multiplier: "1" })),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(m["points.boosters.multiplierLabel"]({ multiplier: "1.25" })),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Jun 9, 15:30")).toBeInTheDocument();
    expect(screen.getAllByText(m["points.boosters.locked"]())).toHaveLength(3);
  });

  it("keeps owned boosters visible but locks expiration when the backend end date is invalid", () => {
    render(
      <BoostersSection
        currentEpoch={9}
        currentEpochEndsAt={new Date("invalid-date")}
        huntedBoosters={[
          huntedBooster({
            epoch: 9,
            loot: "silver_shell",
            multiplier: "1.75",
          }),
        ]}
      />,
    );

    expect(
      screen.getByText(m["points.boosters.multiplierLabel"]({ multiplier: "1.75" })),
    ).toBeInTheDocument();
    expect(
      screen.getByAltText(m["points.boosters.multiplierLabel"]({ multiplier: "1.75" })),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(m["points.boosters.multiplierLabel"]({ multiplier: "1.5" })),
    ).not.toBeInTheDocument();
    expect(screen.getAllByText(m["points.boosters.locked"]())).toHaveLength(4);
  });

  it("formats owned booster expirations in the selected app timezone", () => {
    setAppletsKitUseApp({
      settings: {
        formatNumberOptions: {
          language: "en-US",
        },
        timeZone: "America/New_York",
      },
    });

    render(
      <BoostersSection
        currentEpoch={9}
        currentEpochEndsAt={new Date("2026-06-09T15:30:00Z")}
        huntedBoosters={[
          huntedBooster({
            epoch: 9,
            loot: "bronze_shell",
            multiplier: "1.25",
          }),
        ]}
      />,
    );

    expect(screen.getByText("Jun 9, 11:30")).toBeInTheDocument();
  });
});
