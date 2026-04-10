import type React from "react";
import { useMemo } from "react";
import { Decimal, formatNumber } from "@left-curve/dango/utils";
import { FormattedNumber, Tooltip, twMerge } from "@left-curve/applets-kit";
import { useApp, useCountdown } from "@left-curve/foundation";
import {
  perpsPairStateStore,
  usePerpsState,
  perpsStateStore,
  usePerpsParam,
} from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const FundingCountdown: React.FC = () => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const pairState = perpsPairStateStore((s) => s.pairState);

  usePerpsState();
  const perpsState = perpsStateStore((s) => s.state);

  const { data: perpsParam } = usePerpsParam();

  const countdownEndTime = useMemo(() => {
    if (!perpsState?.lastFundingTime || !perpsParam?.fundingPeriod) {
      return undefined;
    }

    const lastFundingMs = Number(perpsState.lastFundingTime) * 1000;
    const fundingPeriodMs = perpsParam.fundingPeriod * 1000;

    return lastFundingMs + fundingPeriodMs;
  }, [perpsState?.lastFundingTime, perpsParam?.fundingPeriod]);

  const countdown = useCountdown({
    date: countdownEndTime,
    showLeadingZeros: true,
  });

  const { dailyRate, percentValue, annualizedPercent, isPositive } = useMemo(() => {
    if (!pairState?.fundingRate) {
      return { dailyRate: null, percentValue: null, annualizedPercent: null, isPositive: true };
    }

    const rate = Decimal(pairState.fundingRate);

    return {
      dailyRate: rate.toString(),
      percentValue: rate.mul(100).toString(),
      annualizedPercent: rate.mul(100).mul(365).toString(),
      isPositive: rate.gte(0),
    };
  }, [pairState]);

  const formattedCountdown = `${countdown.hours}:${countdown.minutes}:${countdown.seconds}`;

  return (
    <div className="flex gap-1 flex-col items-start lg:min-w-[4rem]">
      <p className="diatype-xs-medium text-ink-tertiary-500">{m["dex.protrade.spot.funding"]()}</p>
      <div className="flex items-baseline gap-2 diatype-xs-medium">
        <Tooltip
          title={
            annualizedPercent
              ? `Annualized: ${formatNumber(annualizedPercent, formatNumberOptions)}%`
              : "Annualized: 0.00%"
          }
        >
          <span
            className={twMerge(
              "diatype-xs-medium cursor-help",
              dailyRate === null
                ? "text-ink-secondary-700"
                : isPositive
                  ? "text-status-success"
                  : "text-status-fail",
            )}
          >
            {percentValue ? (
              <>
                <FormattedNumber number={percentValue} as="span" />%
              </>
            ) : (
              "0.00%"
            )}
          </span>
        </Tooltip>
        <span className="diatype-xs-medium text-ink-secondary-700">{formattedCountdown}</span>
      </div>
    </div>
  );
};
