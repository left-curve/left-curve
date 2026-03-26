import { IconLock, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";

export type NFTRarity = "common" | "uncommon" | "rare" | "epic" | "legendary" | "mythic";

type NFTCardProps = {
  rarity: NFTRarity;
  quantity: number;
  imageSrc: string;
  frameSrc: string;
  className?: string;
  /** When true, shows locked state (e.g., user not logged in) */
  isLocked?: boolean;
};

const RarityLabels: Record<NFTRarity, () => string> = {
  common: () => m["points.rewards.nfts.rarities.common"](),
  uncommon: () => m["points.rewards.nfts.rarities.uncommon"](),
  rare: () => m["points.rewards.nfts.rarities.rare"](),
  epic: () => m["points.rewards.nfts.rarities.epic"](),
  legendary: () => m["points.rewards.nfts.rarities.legendary"](),
  mythic: () => m["points.rewards.nfts.rarities.mythic"](),
};

const RarityColors: Record<NFTRarity, string> = {
  common: "text-[#453d39]",
  uncommon: "text-[#fafafa]",
  rare: "text-[#fafafa]",
  epic: "text-[#fafafa]",
  legendary: "text-[#fafafa]",
  mythic: "text-[#fafafa]",
};

export const NFTCard: React.FC<NFTCardProps> = ({
  rarity,
  quantity,
  imageSrc,
  frameSrc,
  className,
  isLocked = false,
}) => {
  const label = RarityLabels[rarity]();
  const labelColor = RarityColors[rarity];

  return (
    <div
      className={twMerge(
        "flex flex-col gap-1 items-center overflow-hidden pb-2 pt-3 px-7 rounded-[21px] border border-outline-primary-gray bg-surface-secondary-rice max-w-[184px] w-full relative min-h-[11.875rem]",
        className,
      )}
    >
      <div
        className={twMerge(
          "bg-[#261c0a] h-[130px] w-[120px] rounded-lg relative overflow-hidden",
          isLocked && "opacity-50",
        )}
      >
        <div className="absolute inset-0 flex items-center justify-center">
          <img
            src={imageSrc}
            alt={`${label} NFT`}
            className="w-[92px] h-[92px] object-cover select-none drag-none [filter:drop-shadow(0px_4px_100px_rgba(227,189,102,0.5))_drop-shadow(0px_1px_24px_rgba(220,165,67,0.3))]"
          />
        </div>
        <img
          src={frameSrc}
          alt=""
          className="absolute inset-0 w-full h-full object-cover pointer-events-none"
        />
        {/* <p
          className={twMerge(
            "absolute top-[5px] left-1/2 -translate-x-1/2 exposure-xs-italic text-center w-[76px]",
            labelColor,
          )}
        >
          {label}
        </p> */}
      </div>
      {isLocked ? (
        <div className="flex items-center justify-center rounded-full bg-surface-tertiary-gray absolute bottom-[10px] right-[11px] p-1 shadow-account-card z-10">
          <IconLock className="w-6 h-6 text-utility-warning-600" />
        </div>
      ) : (
        <p className="diatype-lg-bold lg:h4-bold text-ink-primary-900">x{quantity}</p>
      )}
    </div>
  );
};
