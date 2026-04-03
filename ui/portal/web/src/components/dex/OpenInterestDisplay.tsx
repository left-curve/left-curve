import type React from "react";
import { useMemo } from "react";
import { Decimal, formatNumber } from "@left-curve/dango/utils";
import { IconToastInfo, Tooltip, twMerge, useApp } from "@left-curve/applets-kit";
import {
  useCurrentPrice,
  usePerpsPairState,
  perpsPairStateStore,
  usePerpsPairParam,
} from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type OpenInterestDisplayProps = {
  pairId: string;
};

export const OpenInterestDisplay: React.FC<OpenInterestDisplayProps> = ({ pairId }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  // Subscribe to pair state updates
  usePerpsPairState({ pairId });
  const pairState = perpsPairStateStore((s) => s.pairState);

  // Get current price for USD conversion
  const { currentPrice } = useCurrentPrice();

  // Get pair params for OI limit
  const { data: pairParam } = usePerpsPairParam({ pairId });

  const { longOiUsd, shortOiUsd, totalOiUsd, isAtLimit } = useMemo(() => {
    if (!pairState || !currentPrice) {
      return { longOiUsd: null, shortOiUsd: null, totalOiUsd: null, isAtLimit: false };
    }

    const price = Decimal(currentPrice);
    const longOi = Decimal(pairState.longOi);
    const shortOi = Decimal(pairState.shortOi);

    const longOiUsd = longOi.mul(price);
    const shortOiUsd = shortOi.mul(price);
    const totalOiUsd = longOiUsd.plus(shortOiUsd);

    // Check if OI is at limit
    let isAtLimit = false;
    if (pairParam?.maxAbsOi) {
      const maxOi = Decimal(pairParam.maxAbsOi);
      isAtLimit = longOi.gte(maxOi) || shortOi.gte(maxOi);
    }

    return {
      longOiUsd: longOiUsd.toString(),
      shortOiUsd: shortOiUsd.toString(),
      totalOiUsd: totalOiUsd.toString(),
      isAtLimit,
    };
  }, [pairState, currentPrice, pairParam]);

  const formatOiValue = (value: string | null) => {
    if (!value) return "-";
    return formatNumber(value, {
      ...formatNumberOptions,
      currency: "usd",
      maximumTotalDigits: 6,
    });
  };

  return (
    <div className="flex gap-1 flex-col items-start lg:min-w-[4rem] col-span-3 lg:col-span-1">
      <p className="diatype-xs-medium text-ink-tertiary-500">
        {m["dex.protrade.spot.openInterest"]()}
      </p>
      <div className="flex items-center gap-1">
        <p
          className={twMerge(
            "diatype-sm-bold tabular-nums lining-nums",
            isAtLimit ? "text-status-fail" : "text-ink-secondary-700",
          )}
        >
          {formatOiValue(longOiUsd)} / {formatOiValue(shortOiUsd)} / {formatOiValue(totalOiUsd)}
        </p>
        {isAtLimit && (
          <Tooltip
            title={m["dex.protrade.spot.oiLimitReached"]()}
            description={m["dex.protrade.spot.oiLimitDescription"]()}
          >
            <IconToastInfo className="text-status-fail w-4 h-4" />
          </Tooltip>
        )}
      </div>
    </div>
  );
};
