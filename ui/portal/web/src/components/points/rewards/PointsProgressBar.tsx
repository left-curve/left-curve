import { formatNumber, type FormatNumberOptions } from "@left-curve/dango/utils";
import { twMerge, useApp } from "@left-curve/applets-kit";
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

const CHEST_SHADOWS: Record<TierKey, string> = {
  bronze:
    "[filter:drop-shadow(0px_4px_100px_#C96A1D66)_drop-shadow(0px_1px_24px_#FFA72C4D)]",
  silver:
    "[filter:drop-shadow(0px_4px_100px_#80850680)_drop-shadow(0px_1px_24px_#B8BE0833)]",
  gold: "[filter:drop-shadow(0px_4px_100px_#E3BD6666)_drop-shadow(0px_1px_24px_#DCA54333)]",
  crystal:
    "[filter:drop-shadow(0px_4px_100px_#BCB8EB80)_drop-shadow(0px_1px_24px_#FFFFFF4D)]",
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

  const isReached = (threshold: number) => isConnected && currentVolume >= threshold;

  return (
    <div className={twMerge("w-full flex flex-col gap-4", className)}>
      <div className="relative w-full">
        {/* Chest card row */}
        <div className="relative w-full h-[54px] mb-2">
          {TIER_MILESTONES.map((ms) => {
            const reached = isReached(ms.threshold);
            return (
              <div
                key={ms.key}
                className={twMerge(
                  "absolute bottom-0 w-[50px] h-[54px] lg:w-[61px] rounded-lg overflow-hidden border bg-surface-secondary-rice",
                  reached
                    ? "border-outline-secondary-rice shadow-[0px_4px_6px_rgba(0,0,0,0.04),0px_4px_6px_rgba(0,0,0,0.04)]"
                    : "border-outline-primary-gray",
                )}
                style={{
                  left: `${ms.position}%`,
                  transform:
                    ms.position === 100 ? "translateX(-100%)" : "translateX(-50%)",
                }}
              >
                <img
                  src={`/images/points/boxes/${ms.key}.png`}
                  alt={`${TierLabels[ms.key]()} chest`}
                  className={twMerge(
                    "absolute inset-0 w-full h-full object-contain select-none drag-none scale-125 -translate-y-1",
                    reached && CHEST_SHADOWS[ms.key],
                  )}
                />
                {/* Dark overlay for unreached */}
                {!reached && (
                  <div className="absolute inset-0 bg-black/50" />
                )}
                {/* Inner highlight for reached */}
                {reached && (
                  <div className="absolute inset-0 pointer-events-none rounded-[inherit] shadow-[inset_0px_3px_6px_-2px_rgba(255,255,255,0.64),inset_0px_0px_8px_-2px_rgba(255,255,255,0.48)]" />
                )}
              </div>
            );
          })}
        </div>

        {/* START flag + Progress bar track */}
        <div className="relative w-full flex items-center">
          {/* START flag */}
          <img
            src="/images/points/startFlag.svg"
            alt={m["points.rewards.boxes.start"]()}
            className="absolute -top-[52px] left-0 w-8 lg:w-10 h-auto select-none drag-none z-10 -translate-x-1/4"
          />

          <div className="relative w-full h-3 rounded-full bg-ink-placeholder-400 border border-[#AFB244]">
            {/* Fill */}
            <div className="absolute inset-0 rounded-full overflow-hidden">
              <div
                className="absolute inset-y-0 left-0 rounded-full transition-all duration-300"
                style={{
                  width: `${progressPercent}%`,
                  backgroundImage:
                    "linear-gradient(-1.8deg, #AFB244 26.16%, #F9F8EC 111.55%)",
                }}
              />
            </div>

            {/* Milestone dots */}
            {TIER_MILESTONES.map((ms) => {
              const filled = progressPercent >= ms.position;
              return (
                <div
                  key={ms.key}
                  className={twMerge(
                    "absolute top-1/2 w-3 h-3 lg:w-4 lg:h-4 rounded-full border-2 z-[1]",
                    filled
                      ? "bg-[#AFB244] border-[#AFB244]"
                      : "bg-ink-placeholder-400 border-[#837d7b]",
                  )}
                  style={{
                    left: `${ms.position}%`,
                    transform: "translateX(-50%) translateY(-50%)",
                  }}
                />
              );
            })}

            {/* Character thumb */}
            {isConnected && (
              <img
                src="/images/points/pointBarThumb.png"
                alt="Progress"
                className="absolute -top-4 w-10 h-10 select-none drag-none z-[2]"
                style={{ left: `calc(${progressPercent}% - 20px)` }}
              />
            )}
          </div>
        </div>

        {/* Dollar labels row */}
        <div className="relative w-full h-6 mt-1">
          {MILESTONES.map((ms) => (
            <span
              key={ms.key}
              className={twMerge(
                "absolute diatype-m-bold lg:diatype-lg-bold text-utility-warning-600",
                !isReached(ms.threshold) && ms.key !== "start" && "opacity-50",
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
      </div>

      {/* Motivational message */}
      <p className="diatype-m-bold text-ink-tertiary-500">{remainingLabel}</p>
    </div>
  );
};
