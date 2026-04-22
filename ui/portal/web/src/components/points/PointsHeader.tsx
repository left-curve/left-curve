import {
  FormattedNumber,
  IconFriendshipGroup,
  IconSprout,
  IconSwapMoney,
  Tooltip,
} from "@left-curve/applets-kit";
import { useApp, useCountdown } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useCurrentEpoch, usePredictPoints } from "@left-curve/store";
import type React from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useUserPoints } from "./useUserPoints";

const BLOCK_TIME_MS = 500;

const formatCountdown = (countdown: {
  days: string;
  hours: string;
  minutes: string;
  seconds: string;
}) => {
  const { days, hours, minutes, seconds } = countdown;
  const d = Number(days);
  const h = Number(hours);
  const m = Number(minutes);

  if (d > 0) return `${days}d ${hours}h ${minutes}m ${seconds}s`;
  if (h > 0) return `${hours}h ${minutes}m ${seconds}s`;
  if (m > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
};

type StartsAt = { block: number } | { timestamp: string };

const EpochStartsIn: React.FC<{ startsAt: StartsAt; onRefetch: () => void }> = ({
  startsAt,
  onRefetch,
}) => {
  const { subscriptions } = useApp();
  const [targetDate, setTargetDate] = useState<Date | undefined>(undefined);
  const hasRefetchedRef = useRef(false);

  useEffect(() => {
    if ("timestamp" in startsAt) {
      setTargetDate(new Date(Number(startsAt.timestamp) * 1000));
      hasRefetchedRef.current = false;
      return;
    }

    const targetBlock = startsAt.block;
    const unsubscribe = subscriptions.subscribe("block", {
      listener: ({ blockHeight }) => {
        const blockDiff = Math.max(0, targetBlock - blockHeight);
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

    const isZero =
      countdown.days === "0" &&
      countdown.hours === "0" &&
      countdown.minutes === "0" &&
      countdown.seconds === "0";
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

type PointCardProps = {
  icon: React.ReactNode;
  value: number | string;
  tooltip?: { title: string; description: string };
};

const PointCard: React.FC<PointCardProps> = ({ icon, value, tooltip }) => (
  <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
    {icon}
    <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
      <p className="text-ink-primary-900">
        <FormattedNumber
          number={value}
          formatOptions={{ fractionDigits: 0 }}
          as="span"
        />
      </p>
      <p>{m["points.header.points"]()}</p>
      {tooltip && <Tooltip title={tooltip.title} description={tooltip.description} />}
    </div>
  </div>
);

type PointsBreakdownRowProps = {
  label: string;
  trading: number;
  lp: number;
  referral: number;
};

const PointsBreakdownRow: React.FC<PointsBreakdownRowProps> = ({
  label,
  trading,
  lp,
  referral,
}) => (
  <div className="flex flex-col gap-2 w-full">
    <p className="text-ink-tertiary-500 diatype-s-medium">{label}</p>
    <div className="flex flex-col lg:flex-row gap-4 w-full">
      <PointCard
        icon={<IconSwapMoney />}
        value={trading}
        tooltip={{ title: m["points.header.tradingPoints.title"](), description: m["points.header.tradingPoints.description"]() }}
      />
      <PointCard
        icon={<IconSprout />}
        value={lp}
        tooltip={{ title: m["points.header.lpPoints.title"](), description: m["points.header.lpPoints.description"]() }}
      />
      <PointCard
        icon={<IconFriendshipGroup />}
        value={referral}
        tooltip={{ title: m["points.header.referralPoints.title"](), description: m["points.header.referralPoints.description"]() }}
      />
    </div>
  </div>
);

export const PointsHeader: React.FC = () => {
  const { isConnected, userIndex } = useAccount();
  const { points, volume, rank, tradingPoints, lpPoints, referralPoints } = useUserPoints();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { isStarted, currentEpoch, endDate, startsAt, refetch } = useCurrentEpoch({ pointsUrl });
  const { predictedPoints } = usePredictPoints({ pointsUrl, userIndex, enabled: isStarted && !!userIndex });

  const predicted = useMemo(() => {
    if (!predictedPoints?.stats) return { vault: 0, perps: 0, referral: 0, total: 0 };
    const vault = Number(predictedPoints.stats.points.vault);
    const perps = Number(predictedPoints.stats.points.perps);
    const referral = Number(predictedPoints.stats.points.referral);
    return { vault, perps, referral, total: vault + perps + referral };
  }, [predictedPoints]);

  const hasPredicted = isStarted && predicted.total > 0;

  const countdown = useCountdown({ date: endDate ?? undefined });
  const hasRefetchedRef = useRef(false);

  useEffect(() => {
    hasRefetchedRef.current = false;
  }, [currentEpoch]);

  useEffect(() => {
    if (!isStarted || !endDate) return;

    const isZero =
      countdown.days === "0" &&
      countdown.hours === "0" &&
      countdown.minutes === "0" &&
      countdown.seconds === "0";
    if (isZero && !hasRefetchedRef.current) {
      hasRefetchedRef.current = true;
      refetch();
    }
  }, [isStarted, endDate, countdown, refetch]);

  return (
    <div className="p-4 lg:p-8 lg:pb-[30px] flex flex-col gap-4 rounded-t-xl">
      <div className="w-full rounded-xl bg-surface-tertiary-rice border border-outline-primary-gray p-4 flex flex-col gap-4 items-center lg:flex-row lg:justify-around">
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">
            {isConnected ? (
              <FormattedNumber number={points} formatOptions={{ fractionDigits: 0 }} as="span" />
            ) : (
              "--"
            )}
          </p>
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["points.header.myPoints"]()}</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">
            {isConnected ? (
              <FormattedNumber number={volume} formatOptions={{ currency: "USD" }} as="span" />
            ) : (
              "--"
            )}
          </p>
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["points.header.myVolume"]()}</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">
            {isConnected ? (
              <>
                {"#"}
                <FormattedNumber number={rank} formatOptions={{ fractionDigits: 0 }} as="span" />
              </>
            ) : (
              "--"
            )}
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
      <PointsBreakdownRow
        label={m["points.header.earnedLabel"]()}
        trading={isConnected ? tradingPoints : 0}
        lp={isConnected ? lpPoints : 0}
        referral={isConnected ? referralPoints : 0}
      />
      {hasPredicted && (
        <PointsBreakdownRow
          label={m["points.header.predictedLabel"]()}
          trading={predicted.perps}
          lp={predicted.vault}
          referral={predicted.referral}
        />
      )}
    </div>
  );
};
