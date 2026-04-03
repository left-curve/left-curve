import type React from "react";
import { useMemo } from "react";
import { Decimal, formatNumber } from "@left-curve/dango/utils";
import { twMerge, useApp } from "@left-curve/applets-kit";
import { useCountdown } from "@left-curve/foundation";
import {
  usePerpsPairState,
  perpsPairStateStore,
  usePerpsState,
  perpsStateStore,
  usePerpsParam,
} from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type FundingCountdownProps = {
  pairId: string;
};

export const FundingCountdown: React.FC<FundingCountdownProps> = ({ pairId }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  // Subscribe to pair state updates (for fundingRate when PR is merged)
  usePerpsPairState({ pairId });
  const pairState = perpsPairStateStore((s) => s.pairState);

  // Subscribe to global perps state (for lastFundingTime)
  usePerpsState();
  const perpsState = perpsStateStore((s) => s.state);

  // Get global params (for fundingPeriod)
  const { data: perpsParam } = usePerpsParam();

  // Calculate countdown end time
  const countdownEndTime = useMemo(() => {
    if (!perpsState?.lastFundingTime || !perpsParam?.fundingPeriod) {
      return undefined;
    }

    // lastFundingTime is a decimal string representing seconds (e.g., "1732770602.144737024")
    // fundingPeriod is in seconds
    const lastFundingMs = Number(perpsState.lastFundingTime) * 1000; // Convert from seconds to ms
    const fundingPeriodMs = perpsParam.fundingPeriod * 1000; // Convert seconds to ms

    return lastFundingMs + fundingPeriodMs;
  }, [perpsState?.lastFundingTime, perpsParam?.fundingPeriod]);

  const countdown = useCountdown({
    date: countdownEndTime,
    showLeadingZeros: true,
  });

  // fundingRate is per-day from the backend
  const { dailyRate, isPositive } = useMemo(() => {
    if (!pairState?.fundingRate) {
      return { dailyRate: null, isPositive: true };
    }

    const rate = Decimal(pairState.fundingRate);

    return {
      dailyRate: rate.toString(),
      isPositive: rate.gte(0),
    };
  }, [pairState]);

  const formattedRate = useMemo(() => {
    if (!dailyRate) return "0.00%";

    // Convert to percentage (multiply by 100)
    const percentValue = Decimal(dailyRate).mul(100).toString();

    return `${formatNumber(percentValue, {
      ...formatNumberOptions,
    })}%`;
  }, [dailyRate, formatNumberOptions]);

  const formattedCountdown = `${countdown.hours}:${countdown.minutes}:${countdown.seconds}`;

  return (
    <div className="flex gap-1 flex-col items-start lg:min-w-[4rem]">
      <p className="diatype-xs-medium text-ink-tertiary-500">{m["dex.protrade.spot.funding"]()}</p>
      <div className="flex items-center gap-2">
        <span
          className={twMerge(
            "diatype-xs-medium",
            dailyRate === null
              ? "text-ink-secondary-700"
              : isPositive
                ? "text-status-success"
                : "text-status-fail",
          )}
        >
          {formattedRate}
        </span>
        <span className="diatype-xs-medium text-ink-secondary-700">
          {formattedCountdown}
        </span>
      </div>
    </div>
  );
};
