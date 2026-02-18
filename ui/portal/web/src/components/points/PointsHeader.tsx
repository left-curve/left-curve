import { IconFriendshipGroup, IconSprout, IconSwapMoney, Tooltip } from "@left-curve/applets-kit";
import type React from "react";
import { useUserPoints } from "./useUserPoints";

export const PointsHeader: React.FC = () => {
  const { points, volume, rank, tradingPoints, lpPoints, referralPoints } = useUserPoints();

  const formatNumber = (num: number) => num.toLocaleString();
  const formatCurrency = (num: number) => `$${num.toLocaleString()}`;

  return (
    <div className="p-4 lg:p-8 lg:pb-[30px] flex flex-col gap-4 rounded-t-xl">
      <div className="w-full rounded-xl bg-surface-tertiary-rice border border-outline-primary-gray p-4 flex flex-col gap-4 items-center lg:flex-row lg:justify-around">
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">{formatNumber(points)}</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">My points</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">{formatCurrency(volume)}</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">My volume</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">#{formatNumber(rank)}</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">My rank</p>
        </div>
      </div>
      <div className="flex flex-col lg:flex-row gap-4 w-full">
        <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
          <IconSwapMoney />
          <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
            <p className="text-ink-primary-900">{formatNumber(tradingPoints)}</p>
            <p>Points</p>
            <Tooltip
              title="Trading Points"
              description="Points earned from organic trading activity"
            />
          </div>
        </div>

        <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
          <IconSprout />
          <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
            <p className="text-ink-primary-900">{formatNumber(lpPoints)}</p>
            <p>Points</p>
            <Tooltip title="LP Points" description="Points earned from providing liquidity" />
          </div>
        </div>
        <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
          <IconFriendshipGroup />
          <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
            <p className="text-ink-primary-900">{formatNumber(referralPoints)}</p>
            <p>Points</p>
            <Tooltip
              title="Referral Points"
              description="Points earned from referring other users"
            />
          </div>
        </div>
      </div>
    </div>
  );
};
