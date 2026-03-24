import { formatNumber, type FormatNumberOptions } from "@left-curve/dango/utils";
import { ProgressBar, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount } from "@left-curve/store";
import type React from "react";

type TierKey = "bronze" | "silver" | "gold" | "crystal";

type Tier = {
  key: TierKey;
  threshold: number;
};

const TierLabels: Record<TierKey, () => string> = {
  bronze: () => m["points.rewards.boxes.tiers.bronze"](),
  silver: () => m["points.rewards.boxes.tiers.silver"](),
  gold: () => m["points.rewards.boxes.tiers.gold"](),
  crystal: () => m["points.rewards.boxes.tiers.crystal"](),
};

const TIERS: Tier[] = [
  { key: "bronze", threshold: 25000 },
  { key: "silver", threshold: 100000 },
  { key: "gold", threshold: 250000 },
  { key: "crystal", threshold: 500000 },
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
  const { isConnected } = useAccount();
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

  const tierLabel = TierLabels[tier.key]();
  const nextTargetLabel = isConnected ? formatUsd(target, { maximumTotalDigits: 3 }) : "--";
  const remainingLabel = isConnected
    ? m["points.rewards.boxes.volumeUntilNext"]({
        amount: formatUsd(remaining, { maximumTotalDigits: integerDigits(remaining) }),
        tier: tierLabel,
      })
    : m["points.rewards.boxes.notLoggedIn"]();

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
