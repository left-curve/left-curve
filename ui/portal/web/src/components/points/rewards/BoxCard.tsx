import {
  Badge,
  Button,
  IconInfo,
  IconLock,
  Tooltip,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";
import { useChestOpening } from "./useChestOpening";

export type BoxVariant = "bronze" | "silver" | "gold" | "crystal";

const VariantLabels: Record<BoxVariant, () => string> = {
  bronze: () => m["points.rewards.boxes.tiers.bronze"](),
  silver: () => m["points.rewards.boxes.tiers.silver"](),
  gold: () => m["points.rewards.boxes.tiers.gold"](),
  crystal: () => m["points.rewards.boxes.tiers.crystal"](),
};

const VariantTooltips: Record<BoxVariant, () => string> = {
  bronze: () => m["points.rewards.boxes.tooltips.bronze"](),
  silver: () => m["points.rewards.boxes.tooltips.silver"](),
  gold: () => m["points.rewards.boxes.tooltips.gold"](),
  crystal: () => m["points.rewards.boxes.tooltips.crystal"](),
};

const VariantConfig: Record<
  BoxVariant,
  {
    badgeColor: "red" | "green" | "rice" | "blue";
    imageShadow: string;
  }
> = {
  bronze: {
    badgeColor: "red",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#C96A1D66)_drop-shadow(0px_1px_24px_#FFA72C4D)]",
  },
  silver: {
    badgeColor: "green",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#80850680)_drop-shadow(0px_1px_24px_#B8BE0833)]",
  },
  gold: {
    badgeColor: "rice",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#E3BD6666)_drop-shadow(0px_1px_24px_#DCA54333)]",
  },
  crystal: {
    badgeColor: "blue",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#BCB8EB80)_drop-shadow(0px_1px_24px_#FFFFFF4D)]",
  },
};

type BoxCardProps = {
  variant: BoxVariant;
  quantity: number;
  className?: string;
  isUserLocked?: boolean;
};

export const BoxCard: React.FC<BoxCardProps> = ({
  className,
  quantity,
  variant,
  isUserLocked = false,
}) => {
  const { isLg } = useMediaQuery();
  const { openChest, openAllChests } = useChestOpening();
  const { badgeColor, imageShadow } = VariantConfig[variant];
  const label = VariantLabels[variant]();
  const tooltip = VariantTooltips[variant]();

  const isLocked = isUserLocked || quantity < 1;

  const handleClick = () => {
    if (isLocked) return;
    openChest(variant);
  };

  const handleOpenAllClick = () => {
    if (isLocked) return;
    openAllChests(variant);
  };

  return (
    <div className={twMerge("flex flex-col items-center gap-3 lg:gap-4", className)}>
      <div className="relative">
        <Badge
          size="m"
          color={badgeColor}
          text={
            <div className="flex pl-1 items-center gap-1">
              {label}
              <Tooltip className="max-w-[21rem]" content={tooltip} placement="top">
                <IconInfo className="inline-block w-4 h-4" />
              </Tooltip>
            </div>
          }
          className="absolute top-2 left-2 z-20 capitalize rounded-full"
        />
        <div className="relative w-[9.6rem] h-[9.6rem] lg:w-[11.6rem] lg:h-[11.9rem] rounded-xl border border-outline-primary-gray bg-surface-secondary-rice shadow-account-card overflow-hidden">
          <img
            src={`/images/points/boxes/${variant}.png`}
            alt={`${label} chest`}
            className={twMerge(
              "w-full h-full object-contain select-none drag-none absolute inset-0 -top-4 lg:-top-6",
              isLocked ? "opacity-50" : imageShadow,
            )}
          />
        </div>
        {isLocked ? (
          <div className="flex items-center justify-center rounded-full bg-surface-tertiary-gray absolute bottom-2 right-2 w-8 h-8 z-10">
            <IconLock className=" w-6 h-6 text-utility-warning-600" />
          </div>
        ) : (
          <p className="absolute bottom-2 left-1/2 -translate-x-1/2 diatype-lg-bold text-ink-primary-900 z-10">
            x{quantity}
          </p>
        )}
      </div>
      <div className="flex flex-col gap-2">
        <Button
          size={isLg ? "md" : "sm"}
          className="px-8 lg:px-10"
          variant="primary"
          isDisabled={isLocked}
          onClick={handleClick}
        >
          {m["points.rewards.boxes.open"]()}
        </Button>
        <Button
          size={isLg ? "md" : "sm"}
          className="px-6 lg:px-8"
          variant="tertiary"
          isDisabled={isLocked || quantity <= 1}
          onClick={handleOpenAllClick}
        >
          {m["points.rewards.boxes.openAll"]()}
        </Button>
      </div>
    </div>
  );
};
