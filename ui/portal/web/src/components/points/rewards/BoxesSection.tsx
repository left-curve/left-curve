import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, type BoxReward } from "@left-curve/store";
import type React from "react";
import { BoxCard, type BoxVariant } from "./BoxCard";

type BoxesSectionProps = {
  unopenedBoxes: Record<string, BoxReward[]>;
  onOpenChest: (variant: BoxVariant) => void;
  onOpenAllChests: (variant: BoxVariant) => void;
};

const VARIANTS: BoxVariant[] = ["bronze", "silver", "gold", "crystal"];

export const BoxesSection: React.FC<BoxesSectionProps> = ({
  unopenedBoxes,
  onOpenChest,
  onOpenAllChests,
}) => {
  const { isConnected } = useAccount();

  return (
    <div className="flex flex-col gap-3">
      <p className="h3-bold text-ink-primary-900">{m["points.rewards.boxes.title"]()}</p>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
        {VARIANTS.map((variant) => (
          <BoxCard
            key={variant}
            variant={variant}
            quantity={unopenedBoxes[variant]?.length ?? 0}
            onClick={() => onOpenChest(variant)}
            onOpenAll={() => onOpenAllChests(variant)}
            isUserLocked={!isConnected}
          />
        ))}
      </div>
    </div>
  );
};
