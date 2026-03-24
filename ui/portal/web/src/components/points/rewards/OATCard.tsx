import { IconFlash, IconTimer, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";

export type OATType = "hurrah" | "trader" | "wizard" | "supporter";

const OATImages: Record<OATType, string> = {
  hurrah: "/images/points/oats/hurrah.png",
  trader: "/images/points/oats/trader.png",
  wizard: "/images/points/oats/wizard.png",
  supporter: "/images/points/oats/supporter.png",
};

const OATTitles: Record<OATType, () => string> = {
  hurrah: () => m["points.boosters.oats.hurrah"](),
  trader: () => m["points.boosters.oats.trader"](),
  wizard: () => m["points.boosters.oats.wizard"](),
  supporter: () => m["points.boosters.oats.supporter"](),
};

/**
 * Format a Unix timestamp (seconds) to MM/DD/YYYY
 */
function formatExpirationDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const year = date.getFullYear();
  return `${month}/${day}/${year}`;
}

type OATCardProps = {
  type: OATType;
  isLocked?: boolean;
  /** Unix timestamp (seconds) when this OAT expires */
  expiresAt?: number;
  /** Points boost percentage */
  pointsBoost?: number;
  className?: string;
};

export const OATCard: React.FC<OATCardProps> = ({
  type,
  isLocked = false,
  expiresAt,
  pointsBoost = 100,
  className,
}) => {
  const title = OATTitles[type]();
  const imageSrc = OATImages[type];
  const expirationDisplay = expiresAt ? formatExpirationDate(expiresAt) : "--";

  return (
    <div
      className={twMerge(
        "flex flex-col rounded-xl overflow-hidden bg-surface-tertiary-gray shadow-account-card p-2 gap-2",
        isLocked && "opacity-50",
        className,
      )}
    >
      <div className="relative">
        <img
          src={imageSrc}
          alt={title}
          className="w-full aspect-square object-cover select-none drag-none rounded-xl"
        />
      </div>
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2 px-2 py-1 bg-surface-disabled-gray rounded-md">
          <IconFlash className="w-4 h-4 text-primitives-green-light-400" />
          <span className="diatype-xs-regular text-ink-primary-900">
            {m["points.boosters.pointsBoost"]({ pointsBoost: String(pointsBoost) })}
          </span>
        </div>
        <div className="flex items-center gap-2 px-2 py-1 bg-surface-disabled-gray rounded-md">
          <IconTimer className="w-4 h-4 text-brand-red-bean" />
          <span className="diatype-xs-regular text-ink-primary-900">{expirationDisplay}</span>
        </div>
      </div>
    </div>
  );
};
