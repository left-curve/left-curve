import type { HuntedBooster, HuntedLoot } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";

import { BoosterCard } from "./BoosterCard";

const TIER_ORDER: HuntedLoot[] = ["bronze_shell", "silver_shell", "golden_shell", "pearl_dango"];

type BoostersSectionProps = {
  huntedBoosters: HuntedBooster[];
  currentEpoch: number | null;
  currentEpochEndsAt: Date | null;
};

export const BoostersSection: React.FC<BoostersSectionProps> = ({
  huntedBoosters,
  currentEpoch,
  currentEpochEndsAt,
}) => {
  const byTier = new Map<HuntedLoot, HuntedBooster>();
  for (const booster of huntedBoosters) {
    if (currentEpoch !== null && booster.epoch !== currentEpoch) continue;
    byTier.set(booster.loot, booster);
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <p className="h4-bold text-ink-primary-900">{m["points.boosters.title"]()}</p>
        <p className="diatype-m-medium text-ink-tertiary-500">
          {m["points.boosters.description"]()}
        </p>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
        {TIER_ORDER.map((loot) => {
          const owned = byTier.get(loot);
          return (
            <BoosterCard
              key={loot}
              loot={loot}
              multiplier={owned?.multiplier.toString()}
              endsAt={owned && currentEpochEndsAt ? currentEpochEndsAt : undefined}
            />
          );
        })}
      </div>
    </div>
  );
};
