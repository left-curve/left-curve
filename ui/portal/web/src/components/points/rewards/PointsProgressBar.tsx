import { formatNumber } from "@left-curve/dango/utils";
import { twMerge, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount } from "@left-curve/store";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type React from "react";

type TierKey = "bronze" | "silver" | "gold" | "crystal";

type Step = {
  kind: "start" | "milestone";
  threshold: number;
  tierKey: TierKey;
};

type ProgressBarState = {
  steps: Step[];
  progress: number;
  nextTier: TierKey;
  remaining: number;
};

const MILESTONE_STEP = 25_000;
const LOOKAHEAD_STEPS = 24; // milestones rendered ahead of the user (>20 as requested)

const TIER_LABELS: Record<TierKey, () => string> = {
  bronze: () => m["points.rewards.boxes.tiers.bronze"](),
  silver: () => m["points.rewards.boxes.tiers.silver"](),
  gold: () => m["points.rewards.boxes.tiers.gold"](),
  crystal: () => m["points.rewards.boxes.tiers.crystal"](),
};

// At each $25K milestone, the highest tier whose threshold divides the absolute
// threshold wins. Tiers are nested powers (25K | 100K | 250K | 500K), so the
// rule applies identically across cycles: e.g. 750K → gold, 1M → crystal.
const tierKeyForThreshold = (threshold: number): TierKey => {
  if (threshold % 500_000 === 0) return "crystal";
  if (threshold % 250_000 === 0) return "gold";
  if (threshold % 100_000 === 0) return "silver";
  return "bronze";
};

const formatThresholdLabel = (value: number): string => {
  if (value === 0) return "$0";

  if (value >= 1_000_000) {
    const millions = value / 1_000_000;
    if (value % 1_000_000 === 0) {
      return `$${millions}M`;
    }
    if (value % 100_000 === 0) {
      return `$${millions.toFixed(1)}M`;
    }
    const formatted = millions.toFixed(3).replace(/\.?0+$/, "");
    return `$${formatted}M`;
  }

  return `$${value / 1000}K`;
};

const calculateSteps = (currentVolume: number): Step[] => {
  const currentMs = Math.floor(Math.max(currentVolume, 0) / MILESTONE_STEP);
  const endMs = currentMs + LOOKAHEAD_STEPS;

  const steps: Step[] = [{ kind: "start", threshold: 0, tierKey: "bronze" }];
  for (let n = 1; n <= endMs; n++) {
    const threshold = n * MILESTONE_STEP;
    steps.push({
      kind: "milestone",
      threshold,
      tierKey: tierKeyForThreshold(threshold),
    });
  }
  return steps;
};

const getNextMilestone = (currentVolume: number): { tier: TierKey; target: number } => {
  const currentMs = Math.floor(Math.max(currentVolume, 0) / MILESTONE_STEP);
  const target = (currentMs + 1) * MILESTONE_STEP;
  return { tier: tierKeyForThreshold(target), target };
};

const useProgressBarState = (currentVolume: number): ProgressBarState => {
  return useMemo(() => {
    const safeVolume = Math.max(currentVolume, 0);
    const steps = calculateSteps(safeVolume);
    const lastThreshold = steps[steps.length - 1].threshold;
    const progress =
      lastThreshold > 0 ? Math.min(Math.max((safeVolume / lastThreshold) * 100, 0), 100) : 0;
    const { tier: nextTier, target } = getNextMilestone(safeVolume);
    const remaining = Math.max(target - safeVolume, 0);

    return { steps, progress, nextTier, remaining };
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
  if (step.kind === "start") {
    return <StartFlag className={flagClassName} />;
  }
  return <ChestCard tierKey={step.tierKey} isReached={isReached} />;
};

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

const STEP_PX = 170;
const EDGE_PX = 25;
const BAR_LEFT_EXTEND = 24;
const BAR_RIGHT_FADE = 60;

type HorizontalProgressBarProps = {
  steps: Step[];
  progress: number;
  currentVolume: number;
};

const HorizontalProgressBar: React.FC<HorizontalProgressBarProps> = ({
  steps,
  progress,
  currentVolume,
}) => {
  const viewportRef = useRef<HTMLDivElement>(null);
  const isDraggingRef = useRef(false);
  const dragStartXRef = useRef(0);
  const dragStartScrollRef = useRef(0);
  const hasAutoCenteredRef = useRef(false);
  const [isAtStart, setIsAtStart] = useState(true);

  const innerWidth = (steps.length - 1) * STEP_PX;
  const trackWidth = innerWidth + EDGE_PX * 2;
  const fillWidth = (progress / 100) * innerWidth + BAR_LEFT_EXTEND;

  const centerOnThumb = useCallback(
    (smooth: boolean) => {
      const viewport = viewportRef.current;
      if (!viewport || isDraggingRef.current) return;
      if (viewport.clientWidth === 0) return;

      const thumbX = EDGE_PX + (progress / 100) * innerWidth;
      const target = thumbX - viewport.clientWidth / 2;
      const maxScroll = viewport.scrollWidth - viewport.clientWidth;
      const clamped = Math.max(0, Math.min(maxScroll, target));

      viewport.scrollTo({ left: clamped, behavior: smooth ? "smooth" : "auto" });
    },
    [progress, innerWidth],
  );

  useEffect(() => {
    centerOnThumb(hasAutoCenteredRef.current);
    hasAutoCenteredRef.current = true;
  }, [centerOnThumb]);

  // Re-center when the page becomes visible again, when the window regains
  // focus, or when the bar scrolls back into view (covers tab switching, SPA
  // navigation that keeps the component mounted, and viewport restoration).
  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;

    const recenter = () => centerOnThumb(false);

    const onVisibility = () => {
      if (document.visibilityState === "visible") recenter();
    };
    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("focus", recenter);

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) recenter();
        }
      },
      { threshold: 0.1 },
    );
    observer.observe(viewport);

    return () => {
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("focus", recenter);
      observer.disconnect();
    };
  }, [centerOnThumb]);

  const onScroll = () => {
    const viewport = viewportRef.current;
    if (!viewport) return;
    setIsAtStart(viewport.scrollLeft <= 1);
  };

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.pointerType !== "mouse") return;
    const viewport = viewportRef.current;
    if (!viewport) return;
    isDraggingRef.current = true;
    dragStartXRef.current = e.clientX;
    dragStartScrollRef.current = viewport.scrollLeft;
    viewport.setPointerCapture(e.pointerId);
  };

  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!isDraggingRef.current) return;
    const viewport = viewportRef.current;
    if (!viewport) return;
    const dx = e.clientX - dragStartXRef.current;
    viewport.scrollLeft = dragStartScrollRef.current - dx;
  };

  const onPointerEnd = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!isDraggingRef.current) return;
    isDraggingRef.current = false;
    viewportRef.current?.releasePointerCapture(e.pointerId);
  };

  return (
    <div
      ref={viewportRef}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
      onScroll={onScroll}
      className="w-full overflow-x-auto overflow-y-hidden overscroll-x-contain touch-pan-x select-none cursor-grab active:cursor-grabbing [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
      style={{
        maskImage: isAtStart
          ? "linear-gradient(to right, black 0, black calc(100% - 24px), transparent 100%)"
          : "linear-gradient(to right, transparent 0, black 24px, black calc(100% - 24px), transparent 100%)",
        WebkitMaskImage: isAtStart
          ? "linear-gradient(to right, black 0, black calc(100% - 24px), transparent 100%)"
          : "linear-gradient(to right, transparent 0, black 24px, black calc(100% - 24px), transparent 100%)",
      }}
    >
      <div className="relative pt-6" style={{ width: trackWidth }}>
        <div className="relative h-[54px]">
          {steps.map((step, idx) => {
            const isReached = currentVolume >= step.threshold;
            return (
              <div
                key={`chest-${step.threshold}`}
                className={twMerge("absolute bottom-0 -translate-x-1/2", {
                  "-bottom-2": idx === 0,
                })}
                style={{ left: idx * STEP_PX + EDGE_PX }}
              >
                <StepIcon step={step} isReached={isReached} flagClassName="w-[44px] h-[72px]" />
              </div>
            );
          })}
        </div>

        <div className="relative h-8">
          <div className="absolute inset-y-0" style={{ left: EDGE_PX, right: EDGE_PX }}>
            <div
              className="absolute top-1/2 -translate-y-1/2 h-3 bg-ink-placeholder-400 border border-brand-green rounded-full overflow-hidden"
              style={{
                left: -BAR_LEFT_EXTEND,
                right: -BAR_RIGHT_FADE,
                maskImage: `linear-gradient(to right, black 0, black calc(100% - ${BAR_RIGHT_FADE}px), transparent 100%)`,
                WebkitMaskImage: `linear-gradient(to right, black 0, black calc(100% - ${BAR_RIGHT_FADE}px), transparent 100%)`,
              }}
            >
              <div
                className="absolute inset-y-0 left-0 rounded-full bg-[linear-gradient(321.22deg,_#AFB244_26.16%,_#F9F8EC_111.55%)] transition-all duration-300"
                style={{ width: `${fillWidth}px` }}
              />
            </div>
            {steps.map((step, idx) => {
              if (step.kind === "start") return null;
              const isReached = currentVolume >= step.threshold;
              const pct = (idx / (steps.length - 1)) * 100;
              return (
                <div
                  key={`mark-${step.threshold}`}
                  className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2"
                  style={{ left: `${pct}%` }}
                >
                  <StepMarker isReached={isReached} />
                </div>
              );
            })}
            <ProgressThumb progress={progress} />
          </div>
        </div>

        <div className="relative h-6">
          {steps.map((step, idx) => {
            const isReached = currentVolume >= step.threshold;
            return (
              <p
                key={`label-${step.threshold}`}
                className={twMerge(
                  "absolute top-0 -translate-x-1/2 diatype-lg-bold text-utility-warning-600 whitespace-nowrap",
                  !isReached && "opacity-50",
                )}
                style={{ left: idx * STEP_PX + EDGE_PX }}
              >
                {formatThresholdLabel(step.threshold)}
              </p>
            );
          })}
        </div>
      </div>
    </div>
  );
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

  const { steps, progress, nextTier, remaining } = useProgressBarState(currentVolume);

  const formatUsd = (value: number) =>
    formatNumber(value, {
      ...formatNumberOptions,
      currency: "USD",
    });

  const tierLabel = TIER_LABELS[nextTier]();
  const remainingLabel = isConnected
    ? m["points.rewards.boxes.volumeUntilNext"]({
        amount: formatUsd(remaining),
        tier: tierLabel,
      })
    : m["points.rewards.boxes.notLoggedIn"]();

  return (
    <div className={twMerge("w-full flex flex-col gap-2", className)}>
      <HorizontalProgressBar steps={steps} progress={progress} currentVolume={currentVolume} />
      <p className="diatype-m-bold text-ink-tertiary-500 mt-2">{remainingLabel}</p>
    </div>
  );
};
