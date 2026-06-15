import type React from "react";
import { useMemo } from "react";
import { Decimal } from "@left-curve/utils";
import { FormattedNumber, IconToastInfo, Tooltip, twMerge } from "@left-curve/applets-kit";
import { useCurrentPrice, usePerpsPairState, usePerpsPairParam } from "@left-curve/store";
import { useProTrade } from "./ProTrade";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const OpenInterestDisplay: React.FC = () => {
  const { pair } = useProTrade();
  const pairId = pair.id;
  const pairState = usePerpsPairState((s) => s.pairState, { pairId });

  const { currentPrice } = useCurrentPrice({ pairId });

  const { data: pairParam } = usePerpsPairParam({ pairId: pairId });

  const { totalOiUsd, isAtLimit } = useMemo(() => {
    if (!pairState || !currentPrice) {
      return { totalOiUsd: null, isAtLimit: false };
    }

    const price = Decimal(currentPrice);
    const longOi = Decimal(pairState.longOi);
    const shortOi = Decimal(pairState.shortOi);

    const totalOiUsd = longOi.mul(price).plus(shortOi.mul(price));

    // Check if OI is at limit
    let isAtLimit = false;
    if (pairParam?.maxAbsOi) {
      const maxOi = Decimal(pairParam.maxAbsOi);
      isAtLimit = longOi.gte(maxOi) || shortOi.gte(maxOi);
    }

    return {
      totalOiUsd: totalOiUsd.toString(),
      isAtLimit,
    };
  }, [pairState, currentPrice, pairParam]);

  const OiValue: React.FC<{ value: string | null }> = ({ value }) => {
    if (!value) return "-";
    return <FormattedNumber number={value} formatOptions={{ currency: "USD" }} as="span" />;
  };

  return (
    <div className="flex gap-1 flex-col items-start lg:w-[5.5rem] lg:shrink-0">
      <Tooltip title="The sum of the notional values of all long and short positions.">
        <p className="diatype-xxs-medium lg:diatype-xs-medium text-ink-tertiary-500 cursor-help underline decoration-dashed underline-offset-[4px] decoration-current">
          {m["dex.protrade.spot.openInterest"]()}
        </p>
      </Tooltip>
      <div className="flex items-center gap-1 diatype-xs-medium text-ink-secondary-700 h-[16.8px]">
        <p
          className={twMerge(
            "diatype-xs-medium",
            isAtLimit ? "text-status-fail" : "text-ink-secondary-700",
          )}
        >
          <OiValue value={totalOiUsd} />
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
