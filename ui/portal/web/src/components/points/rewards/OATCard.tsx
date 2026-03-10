import { IconLock, twMerge } from "@left-curve/applets-kit";
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

type OATCardProps = {
  type: OATType;
  isLocked?: boolean;
  className?: string;
};

export const OATCard: React.FC<OATCardProps> = ({ type, isLocked = false, className }) => {
  const { title, imageSrc } = OATConfig[type];

  return (
    <div
      className={twMerge(
        "relative rounded-xl overflow-hidden aspect-square",
        isLocked && "opacity-70",
        className,
      )}
    >
      <img
        src={imageSrc}
        alt={title}
        className="w-full h-full object-cover select-none drag-none"
      />
      {isLocked && (
        <div className="flex items-center justify-center rounded-full bg-surface-tertiary-gray absolute bottom-3 right-3 w-8 h-8 z-10">
          <IconLock className="w-6 h-6 text-utility-warning-600" />
        </div>
      )}
    </div>
  );
};
