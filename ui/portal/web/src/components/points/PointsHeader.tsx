import {
  IconFriendshipGroup,
  IconSprout,
  IconSwapMoney,
  Tooltip,
} from "@left-curve/applets-kit";
import { useApp, useCountdown } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useCurrentEpoch } from "@left-curve/store";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { useUserPoints } from "./useUserPoints";

const BLOCK_TIME_MS = 500;

const formatCountdown = (countdown: { days: string; hours: string; minutes: string; seconds: string }) => {
  const { days, hours, minutes, seconds } = countdown;
  const d = Number(days);
  const h = Number(hours);
  const m = Number(minutes);

  if (d > 0) return `${days}d ${hours}h ${minutes}m ${seconds}s`;
  if (h > 0) return `${hours}h ${minutes}m ${seconds}s`;
  if (m > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
};

type StartsAt = { Block: number } | { Timestamp: string };

const EpochStartsIn: React.FC<{ startsAt: StartsAt; onRefetch: () => void }> = ({
  startsAt,
  onRefetch,
}) => {
  const { subscriptions } = useApp();
  const [targetDate, setTargetDate] = useState<Date | undefined>(undefined);
  const hasRefetchedRef = useRef(false);

  useEffect(() => {
    if ("Timestamp" in startsAt) {
      setTargetDate(new Date(startsAt.Timestamp));
      hasRefetchedRef.current = false;
      return;
    }

    const unsubscribe = subscriptions.subscribe("block", {
      listener: ({ blockHeight }) => {
        const blockDiff = Math.max(0, startsAt.Block - blockHeight);
        const remainingMs = blockDiff * BLOCK_TIME_MS;
        setTargetDate(new Date(Date.now() + remainingMs));
        hasRefetchedRef.current = false;
      },
    });
    return () => unsubscribe();
  }, [startsAt, subscriptions]);

  const countdown = useCountdown({ date: targetDate });

  useEffect(() => {
    if (!targetDate) return;

    const isZero = countdown.days === "0" && countdown.hours === "0" && countdown.minutes === "0" && countdown.seconds === "0";
    if (isZero && !hasRefetchedRef.current) {
      hasRefetchedRef.current = true;
      onRefetch();
    }
  }, [targetDate, countdown, onRefetch]);

  return (
    <p className="text-ink-tertiary-500 diatype-s-medium">
      {m["points.header.startsIn"]()} {formatCountdown(countdown)}
    </p>
  );
};

export const PointsHeader: React.FC = () => {
  const { isConnected } = useAccount();
  const { points, volume, rank, tradingPoints, lpPoints, referralPoints } = useUserPoints();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { isStarted, currentEpoch, endDate, startsAt, refetch } = useCurrentEpoch({ pointsUrl });

  const countdown = useCountdown({ date: endDate ?? undefined });
  const hasRefetchedRef = useRef(false);

  useEffect(() => {
    hasRefetchedRef.current = false;
  }, [currentEpoch]);

  useEffect(() => {
    if (!isStarted || !endDate) return;

    const isZero = countdown.days === "0" && countdown.hours === "0" && countdown.minutes === "0" && countdown.seconds === "0";
    if (isZero && !hasRefetchedRef.current) {
      hasRefetchedRef.current = true;
      refetch();
    }
  }, [isStarted, endDate, countdown, refetch]);

  const formatNumber = (num: number) => (isConnected ? num.toLocaleString() : "--");
  const formatCurrency = (num: number) => (isConnected ? `$${num.toLocaleString()}` : "--");

  return (
    <div className="p-4 lg:p-8 lg:pb-[30px] flex flex-col gap-4 rounded-t-xl">
      <div className="w-full rounded-xl bg-surface-tertiary-rice border border-outline-primary-gray p-4 flex flex-col gap-4 items-center lg:flex-row lg:justify-around">
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">{formatNumber(points)}</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["points.header.myPoints"]()}</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">{formatCurrency(volume)}</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["points.header.myVolume"]()}</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">
            {isConnected ? `#${rank.toLocaleString()}` : "--"}
          </p>
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["points.header.myRank"]()}</p>
        </div>
        <div className="flex flex-col items-center">
          <div className="flex items-center gap-1">
            <p className="text-ink-secondary-rice h3-bold">
              {m["points.header.currentEpoch"]()} {isStarted ? currentEpoch : "--"}
            </p>
            <Tooltip
              title={m["points.header.epoch.title"]()}
              description={m["points.header.epoch.description"]()}
            />
          </div>
          {isStarted && endDate && (
            <p className="text-ink-tertiary-500 diatype-s-medium">
              {m["points.header.endsIn"]()} {formatCountdown(countdown)}
            </p>
          )}
          {!isStarted && startsAt && <EpochStartsIn startsAt={startsAt} onRefetch={refetch} />}
        </div>
      </div>
      <div className="flex flex-col lg:flex-row gap-4 w-full">
        <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
          <IconSwapMoney />
          <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
            <p className="text-ink-primary-900">{formatNumber(tradingPoints)}</p>
            <p>{m["points.header.points"]()}</p>
            <Tooltip
              title={m["points.header.tradingPoints.title"]()}
              description={m["points.header.tradingPoints.description"]()}
            />
          </div>
        </div>

        <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
          <IconSprout />
          <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
            <p className="text-ink-primary-900">{formatNumber(lpPoints)}</p>
            <p>{m["points.header.points"]()}</p>
            <Tooltip
              title={m["points.header.lpPoints.title"]()}
              description={m["points.header.lpPoints.description"]()}
            />
          </div>
        </div>
        <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
          <IconFriendshipGroup />
          <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
            <p className="text-ink-primary-900">{formatNumber(referralPoints)}</p>
            <p>{m["points.header.points"]()}</p>
            <Tooltip
              title={m["points.header.referralPoints.title"]()}
              description={m["points.header.referralPoints.description"]()}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
