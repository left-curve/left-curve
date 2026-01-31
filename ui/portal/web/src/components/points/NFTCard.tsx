import { twMerge } from "@left-curve/applets-kit";
import type React from "react";

type NFTRarity = "common" | "uncommon" | "epic" | "golden" | "legendary" | "rare";

type NFTCardProps = {
  rarity: NFTRarity;
  quantity: number;
  imageSrc: string;
  className?: string;
};

const RarityConfig: Record<NFTRarity, { label: string }> = {
  common: { label: "Common" },
  uncommon: { label: "Uncommon" },
  epic: { label: "Epic" },
  golden: { label: "Golden" },
  legendary: { label: "Legendary" },
  rare: { label: "Rare" },
};

export const NFTCard: React.FC<NFTCardProps> = ({ rarity, quantity, imageSrc, className }) => {
  const { label } = RarityConfig[rarity];

  return (
    <div
      className={twMerge(
        "flex flex-col items-center rounded-xl w-[9.6rem] h-[9.6rem] lg:w-[11.6rem] lg:h-[11.9rem] border border-outline-primary-gray bg-surface-tertiary-green dark:bg-outline-primary-gray gap-1 pt-4",
        className,
      )}
    >
      <div className="bg-surface-primary-rice border border-outline-secondary-gray rounded-lg overflow-hidden w-20 lg:w-[6.75rem] flex flex-col items-center pb-1.5">
        <div className="pt-2.5 lg:pt-3.5 pb-1 px-2 w-full">
          <p className="exposure-sm-italic text-center text-utility-warning-700">{label}</p>
        </div>
        <div className="bg-[#261C0A] rounded-md lg:rounded-lg w-[4.25rem] h-[4.25rem] lg:h-[5.75rem] lg:w-[5.75rem] overflow-hidden flex items-center justify-center">
          <img
            src={imageSrc}
            alt={`${label} NFT`}
            className="w-full h-full object-cover select-none drag-none [filter:drop-shadow(0px_4px_100px_rgba(227,189,102,0.5))_drop-shadow(0px_1px_24px_rgba(220,165,67,0.3))]"
          />
        </div>
      </div>
      <p className="diatype-lg-bold lg:h3-bold text-ink-primary-900">x{quantity}</p>
    </div>
  );
};
