import { formatNumber, type FormatNumberOptions } from "@left-curve/dango/utils";
import { twMerge, useApp, useMediaQuery } from "@left-curve/applets-kit";
import type React from "react";

type Tier = {
  key: "bronze" | "silver" | "gold" | "crystal";
  label: string;
  threshold: number;
};

const TIERS: Tier[] = [
  { key: "bronze", label: "Bronze", threshold: 25000 },
  { key: "silver", label: "Silver", threshold: 100000 },
  { key: "gold", label: "Gold", threshold: 250000 },
  { key: "crystal", label: "Crystal", threshold: 500000 },
];

const getNextTier = (currentVolume: number) => {
  let start = 0;

  for (const tier of TIERS) {
    if (currentVolume < tier.threshold) {
      return { tier, target: tier.threshold, start };
    }
    start = tier.threshold;
  }

  const tier = TIERS[TIERS.length - 1];
  const cycleSize = tier.threshold;
  const cycleIndex = Math.floor(currentVolume / cycleSize);
  const startCycle = cycleIndex * cycleSize;
  const target = startCycle + cycleSize;

  return { tier, target, start: startCycle };
};

type PointsProgressBarProps = {
  currentVolume: number;
  className?: string;
};

export const PointsProgressBar: React.FC<PointsProgressBarProps> = ({
  currentVolume,
  className,
}) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { tier, target, start } = getNextTier(Math.max(currentVolume, 0));
  const remaining = Math.max(target - currentVolume, 0);
  const segmentSize = Math.max(target - start, 1);
  const progress = Math.min(Math.max((currentVolume - start) / segmentSize, 0), 1);

  const formatUsd = (value: number, opts?: Partial<FormatNumberOptions>) =>
    formatNumber(value, {
      ...formatNumberOptions,
      ...opts,
      currency: "USD",
      minimumTotalDigits: 0,
    });

  const integerDigits = (value: number) =>
    Math.max(Math.floor(Math.abs(value)).toString().length, 1);

  const nextTargetLabel = formatUsd(target, { maximumTotalDigits: 3 });
  const remainingLabel = `${formatUsd(remaining, { maximumTotalDigits: integerDigits(remaining) })} volume until next ${tier.label} Box`;

  const thumbOffset = `${progress * 100}%`;

  return (
    <div className={twMerge("w-full flex flex-col gap-3 lg:gap-4", className)}>
      <div className="flex flex-col lg:flex-row lg:items-center items-end gap-3 lg:gap-0 relative">
        <div className="flex flex-col flex-1 justify-between h-full w-full gap-3 lg:gap-0 ">
          <p className="diatype-m-bold text-ink-tertiary-500 lg:absolute bottom-0">
            {remainingLabel}
          </p>
          <div className="flex items-center gap-3 flex-1 w-full">
            <div className="relative flex-1 h-3 rounded-full bg-ink-placeholder-400 border border-brand-green">
              <div className="absolute inset-0 rounded-full overflow-hidden">
                <div
                  className="absolute inset-y-0 left-0 rounded-full bg-[linear-gradient(321.22deg,_#AFB244_26.16%,_#F9F8EC_111.55%)]"
                  style={{ width: `${progress * 100}%` }}
                />
              </div>
              <img
                src="/images/points/pointBarThumb.png"
                alt="Progress"
                className="absolute -top-3 w-8 h-8 select-none drag-none"
                style={{ left: `calc(${thumbOffset} - 16px)` }}
              />
            </div>
          </div>
        </div>
        <div>
          <img
            src="/images/points/boxes/silver.png"
            alt="Silver chest"
            className="w-[4rem] h-auto select-none drag-none hidden lg:block"
          />
          <p className="diatype-lg-bold text-utility-warning-600 self-end">{nextTargetLabel}</p>
        </div>
      </div>
    </div>
  );
};
