import { IconClock, IconFlash, twMerge, useApp } from "@left-curve/applets-kit";
import type { HuntedLoot } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";
import { Image } from "~/components/foundation/Image";

function formatBoostEndsAt(date: Date, locale: string, timeZone: string): string {
  const tz = timeZone === "local" ? undefined : timeZone;
  const dateFmt = new Intl.DateTimeFormat(locale, {
    month: "short",
    day: "numeric",
    timeZone: tz,
  });
  const timeFmt = new Intl.DateTimeFormat(locale, {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
    timeZone: tz,
  });
  return `${dateFmt.format(date)}, ${timeFmt.format(date)}`;
}

const FALLBACK_MULTIPLIER: Record<HuntedLoot, string> = {
  bronze_shell: "1.25",
  silver_shell: "1.5",
  golden_shell: "2",
  pearl_dango: "2.5",
};

const BOOSTER_IMAGE: Record<HuntedLoot, string> = {
  bronze_shell: "/images/points/boost/booster-bronze.png",
  silver_shell: "/images/points/boost/booster-silver.png",
  golden_shell: "/images/points/boost/booster-golden.png",
  pearl_dango: "/images/points/boost/booster-pearl.png",
};

type BoosterCardProps = {
  loot: HuntedLoot;
  multiplier?: string;
  endsAt?: Date;
  className?: string;
};

export const BoosterCard: React.FC<BoosterCardProps> = ({
  loot,
  multiplier,
  endsAt,
  className,
}) => {
  const { settings } = useApp();
  const locale = settings.formatNumberOptions.language;
  const timeZone = settings.timeZone;

  const isLocked = multiplier === undefined;
  const displayMultiplier = multiplier ?? FALLBACK_MULTIPLIER[loot];

  const expirationDisplay = (() => {
    if (!endsAt || Number.isNaN(endsAt.getTime())) return m["points.boosters.locked"]();
    return formatBoostEndsAt(endsAt, locale, timeZone);
  })();

  return (
    <div
      className={twMerge(
        "flex flex-col rounded-xl overflow-hidden bg-surface-secondary-rice border border-outline-primary-gray shadow-account-card p-4 gap-4",
        className,
      )}
    >
      <div className="relative">
        <Image
          src={BOOSTER_IMAGE[loot]}
          alt={m["points.boosters.multiplierLabel"]({ multiplier: displayMultiplier })}
          className={twMerge(
            "w-full aspect-square object-cover select-none drag-none rounded-xl",
            isLocked && "opacity-50",
          )}
        />
      </div>
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-2 px-2 py-1 bg-surface-tertiary-gray rounded-md">
          <IconFlash className="w-6 h-6 text-primitives-green-light-400 flex-shrink-0" />
          <span
            className={twMerge(
              "diatype-xs-regular text-ink-primary-900 ml-auto text-right",
              isLocked && "text-ink-tertiary-500",
            )}
          >
            {m["points.boosters.multiplierLabel"]({ multiplier: displayMultiplier })}
          </span>
        </div>
        <div className="flex items-center gap-2 px-2 py-1 bg-surface-tertiary-gray rounded-md">
          <IconClock className="w-6 h-6 text-fg-primary-red flex-shrink-0" />
          <span
            className={twMerge(
              "diatype-xs-regular text-ink-primary-900 ml-auto text-right",
              isLocked && "text-ink-tertiary-500",
            )}
          >
            {expirationDisplay}
          </span>
        </div>
      </div>
    </div>
  );
};
