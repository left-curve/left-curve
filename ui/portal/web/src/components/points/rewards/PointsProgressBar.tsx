import { formatNumber, type FormatNumberOptions } from "@left-curve/dango/utils";
import { Badge, twMerge, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { REWARD_STEP, TIERS, type TierKey, useAccount } from "@left-curve/store";
import type React from "react";

const TierLabels: Record<TierKey, () => string> = {
  bronze: () => m["points.rewards.boxes.tiers.bronze"](),
  silver: () => m["points.rewards.boxes.tiers.silver"](),
  gold: () => m["points.rewards.boxes.tiers.gold"](),
  crystal: () => m["points.rewards.boxes.tiers.crystal"](),
};

type Milestone = {
  key: "start" | TierKey;
  threshold: number;
  position: number;
};

type TierMilestone = Milestone & { key: TierKey };

const START_MILESTONE: Milestone = { key: "start", threshold: 0, position: 0 };

const TIER_MILESTONES: TierMilestone[] = TIERS.map((tier, i) => ({
  key: tier.key,
  threshold: tier.threshold,
  position: ((i + 1) / TIERS.length) * 100,
}));

const MILESTONES: Milestone[] = [START_MILESTONE, ...TIER_MILESTONES];

const MAX_THRESHOLD = TIERS[TIERS.length - 1].threshold;

const getProgressPercent = (currentVolume: number): number => {
  const clamped = Math.min(Math.max(currentVolume, 0), MAX_THRESHOLD);
  for (let i = 1; i < MILESTONES.length; i++) {
    const prev = MILESTONES[i - 1];
    const curr = MILESTONES[i];
    if (clamped <= curr.threshold) {
      const segment = curr.threshold - prev.threshold;
      const progress = segment > 0 ? (clamped - prev.threshold) / segment : 0;
      return prev.position + progress * (curr.position - prev.position);
    }
  }
  return 100;
};

const getNextReward = (currentVolume: number) => {
  const normalizedVolume = Math.max(currentVolume, 0);
  const target = (Math.floor(normalizedVolume / REWARD_STEP) + 1) * REWARD_STEP;
  const tier =
    [...TIERS].reverse().find(({ threshold }) => target % threshold === 0) ?? TIERS[0];
  return { tier, target };
};

const formatCompact = (value: number): string => {
  if (value >= 1_000) return `$${Math.round(value / 1_000)}K`;
  return `$${value}`;
};

const CHEST_SIZES: Record<TierKey, string> = {
  bronze: "w-10 lg:w-12",
  silver: "w-11 lg:w-14",
  gold: "w-12 lg:w-14",
  crystal: "w-12 lg:w-16",
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

  const progressPercent = isConnected ? getProgressPercent(currentVolume) : 0;
  const { tier: nextTier } = getNextReward(currentVolume);
  const nextTarget = (Math.floor(Math.max(currentVolume, 0) / REWARD_STEP) + 1) * REWARD_STEP;
  const remaining = Math.max(nextTarget - currentVolume, 0);

  const formatUsd = (value: number, opts?: Partial<FormatNumberOptions>) =>
    formatNumber(value, {
      ...formatNumberOptions,
      ...opts,
      currency: "USD",
      minimumTotalDigits: 0,
    });

  const integerDigits = (value: number) =>
    Math.max(Math.floor(Math.abs(value)).toString().length, 1);

  const tierLabel = TierLabels[nextTier.key]();
  const remainingLabel = isConnected
    ? m["points.rewards.boxes.volumeUntilNext"]({
        amount: formatUsd(remaining, { maximumTotalDigits: integerDigits(remaining) }),
        tier: tierLabel,
      })
    : m["points.rewards.boxes.notLoggedIn"]();

  return (
    <div className={twMerge("w-full flex flex-col gap-1", className)}>
      {/* Chest images row */}
      <div className="relative w-full h-10 lg:h-14 mb-1">
        {TIER_MILESTONES.map((ms) => (
          <img
            key={ms.key}
            src={`/images/points/boxes/${ms.key}.png`}
            alt={`${TierLabels[ms.key]()} chest`}
            className={twMerge(
              "absolute bottom-0 select-none drag-none object-contain transition-opacity duration-300",
              CHEST_SIZES[ms.key],
              currentVolume >= ms.threshold || !isConnected ? "opacity-100" : "opacity-50",
            )}
            style={{
              left: `${ms.position}%`,
              transform: ms.position === 100 ? "translateX(-100%)" : "translateX(-50%)",
            }}
          />
        ))}
      </div>

      {/* Progress bar track */}
      <div className="relative w-full flex items-center">
        {/* START badge */}
        <Badge
          size="s"
          color="red"
          text={m["points.rewards.boxes.start"]()}
          className="absolute -top-5 left-0 -translate-x-1/4 z-10 rounded-sm"
        />

        <div className="relative w-full h-3 rounded-full bg-ink-placeholder-400 border border-brand-green">
          {/* Fill */}
          <div className="absolute inset-0 rounded-full overflow-hidden">
            <div
              className="absolute inset-y-0 left-0 rounded-full transition-all duration-300 bg-[linear-gradient(321.22deg,_#AFB244_26.16%,_#F9F8EC_111.55%)]"
              style={{ width: `${progressPercent}%` }}
            />
          </div>

          {/* Milestone markers */}
          {TIER_MILESTONES.map((ms) => (
            <div
              key={ms.key}
              className={twMerge(
                "absolute top-1/2 -translate-y-1/2 w-2 h-2 rounded-full border border-brand-green z-[1]",
                progressPercent >= ms.position
                  ? "bg-[#F9F8EC]"
                  : "bg-ink-placeholder-400",
              )}
              style={{
                left: `${ms.position}%`,
                transform: "translateX(-50%) translateY(-50%)",
              }}
            />
          ))}

          {/* Thumb */}
          {isConnected && (
            <img
              src="/images/points/pointBarThumb.png"
              alt="Progress"
              className="absolute -top-3 w-8 h-8 select-none drag-none z-[2]"
              style={{ left: `calc(${progressPercent}% - 16px)` }}
            />
          )}
        </div>
      </div>

      {/* Dollar labels row */}
      <div className="relative w-full h-5 mt-0.5">
        {MILESTONES.map((ms) => (
          <span
            key={ms.key}
            className={twMerge(
              "absolute diatype-xs-medium lg:diatype-s-medium text-ink-tertiary-500",
              ms.position === 0 && "left-0",
              ms.position === 100 && "right-0",
              ms.position > 0 && ms.position < 100 && "-translate-x-1/2",
            )}
            style={
              ms.position > 0 && ms.position < 100
                ? { left: `${ms.position}%` }
                : undefined
            }
          >
            {formatCompact(ms.threshold)}
          </span>
        ))}
      </div>

      {/* Motivational message */}
      <p className="diatype-m-bold text-ink-tertiary-500 mt-1">{remainingLabel}</p>
    </div>
  );
};
