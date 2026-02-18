import type React from "react";
import { BoxCard, type BoxVariant } from "./BoxCard";

type BoxesSectionProps = {
  volume: number;
  onOpenChest: (variant: BoxVariant) => void;
};

export const BoxesSection: React.FC<BoxesSectionProps> = ({ volume, onOpenChest }) => {
  return (
    <div className="flex flex-col gap-3">
      <p className="h3-bold text-ink-primary-900">My boxes</p>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
        <BoxCard variant="bronze" volume={volume} onClick={() => onOpenChest("bronze")} />
        <BoxCard variant="silver" volume={volume} onClick={() => onOpenChest("silver")} />
        <BoxCard variant="gold" volume={volume} onClick={() => onOpenChest("gold")} />
        <BoxCard variant="crystal" volume={volume} onClick={() => onOpenChest("crystal")} />
      </div>
    </div>
  );
};
