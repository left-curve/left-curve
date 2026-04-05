import { formatNumber, type FormatNumberOptions } from "@left-curve/dango/utils";
import { twMerge, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount } from "@left-curve/store";
import { useMemo } from "react";
import type React from "react";

type TierKey = "bronze" | "silver" | "gold" | "crystal";
type StepKey = TierKey | "start";

type Step = {
  key: StepKey;
  threshold: number;
};

type ProgressBarState = {
  steps: Step[];
  progress: number;
  currentStep: Step;
  nextStep: Step;
  segmentProgress: number;
  nextTier: TierKey;
  remaining: number;
};

const CYCLE_SIZE = 500000;

const TIER_OFFSETS: { key: StepKey; offset: number }[] = [
  { key: "start", offset: 0 },
  { key: "bronze", offset: 25000 },
  { key: "silver", offset: 100000 },
  { key: "gold", offset: 250000 },
  { key: "crystal", offset: 500000 },
];

const TIER_LABELS: Record<TierKey, () => string> = {
  bronze: () => m["points.rewards.boxes.tiers.bronze"](),
  silver: () => m["points.rewards.boxes.tiers.silver"](),
  gold: () => m["points.rewards.boxes.tiers.gold"](),
  crystal: () => m["points.rewards.boxes.tiers.crystal"](),
};

const formatThresholdLabel = (value: number): string => {
  if (value === 0) return "$0";

  if (value >= 1000000) {
    const millions = value / 1000000;
    if (value % 1000000 === 0) {
      return `$${millions}M`;
    }
    if (value % 100000 === 0) {
      return `$${millions.toFixed(1)}M`;
    }
    const formatted = millions.toFixed(3).replace(/\.?0+$/, "");
    return `$${formatted}M`;
  }

  return `$${value / 1000}K`;
};

const getStepPosition = (index: number, totalSteps: number): number => {
  return (index / (totalSteps - 1)) * 100;
};

const calculateCycleSteps = (currentVolume: number): { steps: Step[]; cycleStart: number } => {
  const cycleNumber = Math.floor(currentVolume / CYCLE_SIZE);
  const cycleStart = cycleNumber * CYCLE_SIZE;

  const steps: Step[] = TIER_OFFSETS.map(({ key, offset }) => ({
    key,
    threshold: cycleStart + offset,
  }));

  return { steps, cycleStart };
};

const calculateProgress = (currentVolume: number, cycleStart: number): number => {
  const progressInCycle = currentVolume - cycleStart;
  return Math.min((progressInCycle / CYCLE_SIZE) * 100, 100);
};

const getCurrentAndNextStep = (
  currentVolume: number,
  steps: Step[],
): { currentStep: Step; nextStep: Step; segmentProgress: number } => {
  let currentStep = steps[0];
  let nextStep = steps[1];

  for (let i = steps.length - 1; i >= 0; i--) {
    if (currentVolume >= steps[i].threshold) {
      currentStep = steps[i];
      nextStep = steps[i + 1] || steps[i];
      break;
    }
  }

  const segmentSize = nextStep.threshold - currentStep.threshold;
  const progressInSegment = currentVolume - currentStep.threshold;
  const segmentProgress =
    segmentSize > 0 ? Math.min((progressInSegment / segmentSize) * 100, 100) : 100;

  return { currentStep, nextStep, segmentProgress };
};

const getNextTier = (currentVolume: number, steps: Step[]): { tier: TierKey; target: number } => {
  const nextStep = steps.find((step) => step.key !== "start" && step.threshold > currentVolume);
  if (nextStep && nextStep.key !== "start") {
    return { tier: nextStep.key, target: nextStep.threshold };
  }
  const nextCycleStart = Math.floor(currentVolume / CYCLE_SIZE) * CYCLE_SIZE + CYCLE_SIZE;
  return { tier: "crystal", target: nextCycleStart };
};

const useProgressBarState = (currentVolume: number): ProgressBarState => {
  return useMemo(() => {
    const { steps, cycleStart } = calculateCycleSteps(currentVolume);
    const progress = calculateProgress(currentVolume, cycleStart);
    const { currentStep, nextStep, segmentProgress } = getCurrentAndNextStep(currentVolume, steps);
    const { tier: nextTier, target: nextTarget } = getNextTier(currentVolume, steps);
    const remaining = Math.max(nextTarget - currentVolume, 0);

    return { steps, progress, currentStep, nextStep, segmentProgress, nextTier, remaining };
  }, [currentVolume]);
};

type ChestCardProps = {
  tierKey: TierKey;
  isReached: boolean;
  className?: string;
};

const ChestCard: React.FC<ChestCardProps> = ({ tierKey, isReached, className }) => (
  <div
    className={twMerge(
      "relative w-[61px] h-[54px] rounded-lg overflow-hidden",
      "bg-surface-secondary-rice border",
      isReached
        ? "border-brand-rice shadow-[0px_4px_6px_0px_rgba(0,0,0,0.04),0px_4px_6px_0px_rgba(0,0,0,0.04)]"
        : "border-outline-primary-gray",
      className,
    )}
  >
    <img
      src={`/images/points/boxes/${tierKey}.png`}
      alt={`${tierKey} chest`}
      className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-[65%] size-full scale-150 object-contain pointer-events-none"
    />
    {!isReached && <div className="absolute inset-0 bg-black/50 rounded-lg" />}
    {isReached && (
      <div className="absolute inset-0 pointer-events-none rounded-[inherit] shadow-[inset_0px_3px_6px_-2px_rgba(255,255,255,0.64),inset_0px_0px_8px_-2px_rgba(255,255,255,0.48)]" />
    )}
  </div>
);

const StepMarker: React.FC<{ isReached: boolean }> = ({ isReached }) => (
  <div
    className={twMerge(
      "size-4 rounded-full border bg-brand-green",
      isReached ? "border-outline-primary-rice" : "border-ink-secondary-rice",
    )}
  />
);

const StartFlag: React.FC<{ className?: string }> = ({ className }) => (
  <img
    src="/images/points/points-flag.png"
    alt="Start"
    className={twMerge("object-contain", className)}
  />
);

type StepIconProps = {
  step: Step;
  isReached: boolean;
  flagClassName?: string;
};

const StepIcon: React.FC<StepIconProps> = ({ step, isReached, flagClassName }) => {
  if (step.key === "start") {
    return <StartFlag className={flagClassName} />;
  }
  return <ChestCard tierKey={step.key} isReached={isReached} />;
};

type ProgressTrackProps = {
  progress: number;
};

const ProgressTrack: React.FC<ProgressTrackProps> = ({ progress }) => (
  <div className="absolute top-1/2 -translate-y-1/2 left-0 right-0 h-3 bg-ink-placeholder-400 border border-brand-green rounded-full overflow-hidden">
    <div
      className="absolute inset-y-0 left-0 rounded-full bg-[linear-gradient(321.22deg,_#AFB244_26.16%,_#F9F8EC_111.55%)] transition-all duration-300"
      style={{ width: `${progress}%` }}
    />
  </div>
);

type ProgressThumbProps = {
  progress: number;
};

const ProgressThumb: React.FC<ProgressThumbProps> = ({ progress }) => (
  <img
    src="/images/points/pointBarThumb.png"
    alt="Progress"
    className="absolute top-1/2 -translate-y-1/2 w-10 h-10 select-none pointer-events-none transition-all duration-300 z-10"
    style={{ left: `calc(${progress}% - 20px)` }}
  />
);

type MobileProgressBarProps = {
  currentStep: Step;
  nextStep: Step;
  segmentProgress: number;
};

const MobileProgressBar: React.FC<MobileProgressBarProps> = ({
  currentStep,
  nextStep,
  segmentProgress,
}) => (
  <div className="lg:hidden flex flex-col gap-2">
    <div className="flex justify-between items-end">
      <div className="flex flex-col items-center">
        <StepIcon step={currentStep} isReached={true} flagClassName="w-10 h-12" />
      </div>
      <div className="flex flex-col items-center">
        <StepIcon step={nextStep} isReached={false} flagClassName="w-10 h-12" />
      </div>
    </div>

    <div className="relative w-full h-8">
      <ProgressTrack progress={segmentProgress} />

      <div className="absolute top-1/2 -translate-y-1/2 left-0">
        <StepMarker isReached={true} />
      </div>
      <div className="absolute top-1/2 -translate-y-1/2 right-0">
        <StepMarker isReached={segmentProgress >= 100} />
      </div>

      <ProgressThumb progress={segmentProgress} />
    </div>

    <div className="flex justify-between items-start">
      <p className="diatype-lg-bold text-utility-warning-600">
        {formatThresholdLabel(currentStep.threshold)}
      </p>
      <p className="diatype-lg-bold text-utility-warning-600 opacity-50">
        {formatThresholdLabel(nextStep.threshold)}
      </p>
    </div>
  </div>
);

type DesktopProgressBarProps = {
  steps: Step[];
  progress: number;
  currentVolume: number;
};

const DesktopProgressBar: React.FC<DesktopProgressBarProps> = ({
  steps,
  progress,
  currentVolume,
}) => (
  <div className="hidden lg:flex flex-col gap-2">
    <div className="relative w-full h-[72px]">
      {steps.map((step, index) => {
        const isReached = currentVolume >= step.threshold;
        const position = getStepPosition(index, steps.length);
        const isFirst = index === 0;
        const isLast = index === steps.length - 1;

        return (
          <div
            key={step.key}
            className={twMerge(
              "absolute bottom-0",
              isFirst && "left-0",
              isLast && "right-0 translate-x-[20%]",
              !isFirst && !isLast && "-translate-x-1/2",
            )}
            style={!isFirst && !isLast ? { left: `${position}%` } : undefined}
          >
            <StepIcon
              step={step}
              isReached={isReached}
              flagClassName="w-[44px] h-[72px] -mb-4"
            />
          </div>
        );
      })}
    </div>

    <div className="relative w-full h-8">
      <ProgressTrack progress={progress} />

      {steps.map((step, index) => {
        if (step.key === "start") return null;

        const isReached = currentVolume >= step.threshold;
        const position = getStepPosition(index, steps.length);

        return (
          <div
            key={step.key}
            className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2"
            style={{ left: `${position}%` }}
          >
            <StepMarker isReached={isReached} />
          </div>
        );
      })}

      <ProgressThumb progress={progress} />
    </div>

    <div className="relative w-full h-6">
      {steps.map((step, index) => {
        const isReached = currentVolume >= step.threshold;
        const position = getStepPosition(index, steps.length);
        const isFirst = index === 0;
        const isLast = index === steps.length - 1;

        return (
          <p
            key={step.key}
            className={twMerge(
              "absolute diatype-lg-bold text-utility-warning-600",
              !isReached && "opacity-50",
              isFirst && "left-0",
              isLast && "right-0",
              !isFirst && !isLast && "-translate-x-1/2",
            )}
            style={!isFirst && !isLast ? { left: `${position}%` } : undefined}
          >
            {formatThresholdLabel(step.threshold)}
          </p>
        );
      })}
    </div>
  </div>
);

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

  const { steps, progress, currentStep, nextStep, segmentProgress, nextTier, remaining } =
    useProgressBarState(currentVolume);

  const formatUsd = (value: number) =>
    formatNumber(value, {
      ...formatNumberOptions,
      currency: "USD",
    });

  const integerDigits = (value: number) =>
    Math.max(Math.floor(Math.abs(value)).toString().length, 1);

  const tierLabel = TIER_LABELS[nextTier]();
  const remainingLabel = isConnected
    ? m["points.rewards.boxes.volumeUntilNext"]({
        amount: formatUsd(remaining),
        tier: tierLabel,
      })
    : m["points.rewards.boxes.notLoggedIn"]();

  return (
    <div className={twMerge("w-full flex flex-col gap-2", className)}>
      <MobileProgressBar
        currentStep={currentStep}
        nextStep={nextStep}
        segmentProgress={segmentProgress}
      />

      <DesktopProgressBar steps={steps} progress={progress} currentVolume={currentVolume} />

      <p className="diatype-m-bold text-ink-tertiary-500 mt-2">{remainingLabel}</p>
    </div>
  );
};
