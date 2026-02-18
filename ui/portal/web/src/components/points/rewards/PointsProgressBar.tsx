import { formatNumber, type FormatNumberOptions } from "@left-curve/dango/utils";
import { ProgressBar, useApp } from "@left-curve/applets-kit";
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
  const progress = Math.min(Math.max((currentVolume - start) / segmentSize, 0), 1) * 100;

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

  return (
    <ProgressBar
      progress={progress}
      leftLabel={remainingLabel}
      rightLabel={nextTargetLabel}
      thumbSrc="/images/points/pointBarThumb.png"
      endImageSrc="/images/points/boxes/silver.png"
      endImageAlt="Silver chest"
      className={className}
    />
  );
};
