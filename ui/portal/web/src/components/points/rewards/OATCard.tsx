import { IconFlash, IconTimer, twMerge } from "@left-curve/applets-kit";
import type React from "react";

export type OATType = "hurrah" | "trader" | "wizard" | "supporter";

const OATConfig: Record<
  OATType,
  {
    title: string;
    imageSrc: string;
  }
> = {
  hurrah: {
    title: "The Last Hurrah",
    imageSrc: "/images/points/oats/hurrah.png",
  },
  trader: {
    title: "Testnet Trader",
    imageSrc: "/images/points/oats/trader.png",
  },
  wizard: {
    title: "Testnet Wizard",
    imageSrc: "/images/points/oats/wizard.png",
  },
  supporter: {
    title: "Early Supporter",
    imageSrc: "/images/points/oats/supporter.png",
  },
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
  const { title, imageSrc } = OATConfig[type];
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
          <span className="diatype-xs-regular text-ink-primary-900">+{pointsBoost}% Points</span>
        </div>
        <div className="flex items-center gap-2 px-2 py-1 bg-surface-disabled-gray rounded-md">
          <IconTimer className="w-4 h-4 text-brand-red-bean" />
          <span className="diatype-xs-regular text-ink-primary-900">{expirationDisplay}</span>
        </div>
      </div>
    </div>
  );
};
