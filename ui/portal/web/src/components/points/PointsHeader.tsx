import {
  IconFriendshipGroup,
  IconSprout,
  IconSwapMoney,
  Tooltip,
} from "@left-curve/applets-kit";
import { useApp } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useCurrentEpoch } from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { useUserPoints } from "./useUserPoints";

const BLOCK_TIME_MS = 500;

const formatCountdown = (seconds: number) => {
  const days = Math.floor(seconds / (60 * 60 * 24));
  const hours = Math.floor((seconds % (60 * 60 * 24)) / (60 * 60));
  const minutes = Math.floor((seconds % (60 * 60)) / 60);
  const secs = Math.floor(seconds % 60);

  if (days > 0) return `${days}d ${hours}h ${minutes}m ${secs}s`;
  if (hours > 0) return `${hours}h ${minutes}m ${secs}s`;
  if (minutes > 0) return `${minutes}m ${secs}s`;
  return `${secs}s`;
};

const EpochCountdown: React.FC<{ remainingSeconds: number | null; onRefetch: () => void }> = ({
  remainingSeconds,
  onRefetch,
}) => {
  const [currentRemaining, setCurrentRemaining] = useState(remainingSeconds ?? 0);
  const hasRefetchedRef = useRef(false);

  useEffect(() => {
    setCurrentRemaining(remainingSeconds ?? 0);
    hasRefetchedRef.current = false;
  }, [remainingSeconds]);

  useEffect(() => {
    if (currentRemaining <= 0) {
      if (!hasRefetchedRef.current) {
        hasRefetchedRef.current = true;
        const timeout = setTimeout(() => onRefetch(), 1500);
        return () => clearTimeout(timeout);
      }
      return;
    }

    const timer = setInterval(() => {
      setCurrentRemaining((prev) => Math.max(0, prev - 1));
    }, 1000);

    return () => clearInterval(timer);
  }, [currentRemaining > 0, onRefetch]);

  return (
    <p className="text-ink-tertiary-500 diatype-s-medium">
      {m["points.header.endsIn"]()} {formatCountdown(currentRemaining)}
    </p>
  );
};

type StartsAt = { Block: number } | { Timestamp: string };

const EpochStartsIn: React.FC<{ startsAt: StartsAt; onRefetch: () => void }> = ({
  startsAt,
  onRefetch,
}) => {
  const { subscriptions } = useApp();
  const [currentBlock, setCurrentBlock] = useState<number | null>(null);
  const [remainingSeconds, setRemainingSeconds] = useState<number>(0);
  const hasRefetchedRef = useRef(false);

  useEffect(() => {
    if (!("Block" in startsAt)) return;

    const unsubscribe = subscriptions.subscribe("block", {
      listener: ({ blockHeight }) => {
        setCurrentBlock(blockHeight);
      },
    });
    return () => unsubscribe();
  }, [startsAt, subscriptions]);

  useEffect(() => {
    if ("Timestamp" in startsAt) {
      const targetTime = new Date(startsAt.Timestamp).getTime();
      const remaining = Math.max(0, Math.floor((targetTime - Date.now()) / 1000));
      setRemainingSeconds(remaining);
      hasRefetchedRef.current = false;
    } else if ("Block" in startsAt && currentBlock !== null) {
      const blockDiff = Math.max(0, startsAt.Block - currentBlock);
      const remaining = Math.floor((blockDiff * BLOCK_TIME_MS) / 1000);
      setRemainingSeconds(remaining);
      hasRefetchedRef.current = false;
    }
  }, [startsAt, currentBlock]);

  useEffect(() => {
    if (remainingSeconds <= 0) {
      if (!hasRefetchedRef.current) {
        hasRefetchedRef.current = true;
        const timeout = setTimeout(() => onRefetch(), 1500);
        return () => clearTimeout(timeout);
      }
      return;
    }

    const timer = setInterval(() => {
      setRemainingSeconds((prev) => Math.max(0, prev - 1));
    }, 1000);

    return () => clearInterval(timer);
  }, [remainingSeconds > 0, onRefetch]);

  return (
    <p className="text-ink-tertiary-500 diatype-s-medium">
      {m["points.header.startsIn"]()} {formatCountdown(remainingSeconds)}
    </p>
  );
};

export const PointsHeader: React.FC = () => {
  const { isConnected } = useAccount();
  const { points, volume, rank, tradingPoints, lpPoints, referralPoints } = useUserPoints();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { isStarted, currentEpoch, remainingSeconds, startsAt } = useCurrentEpoch({ pointsUrl });
  const queryClient = useQueryClient();

  const handleEpochRefetch = () => {
    queryClient.invalidateQueries({ queryKey: ["currentEpoch"] });
  };

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
          {isStarted && <EpochCountdown remainingSeconds={remainingSeconds} onRefetch={handleEpochRefetch} />}
          {!isStarted && startsAt && <EpochStartsIn startsAt={startsAt} onRefetch={handleEpochRefetch} />}
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
