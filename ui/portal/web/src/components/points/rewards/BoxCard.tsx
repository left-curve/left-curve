import {
  Badge,
  Button,
  IconInfo,
  IconLock,
  Tooltip,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";
import type React from "react";

export type BoxVariant = "bronze" | "silver" | "gold" | "crystal";

const VariantConfig: Record<
  BoxVariant,
  {
    label: string;
    badgeColor: "red" | "green" | "rice" | "blue";
    tooltip: string;
    imageShadow: string;
    threshold: number;
  }
> = {
  bronze: {
    label: "Bronze",
    badgeColor: "red",
    tooltip: "Receive a Bronze chest for every $25k volume.",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#C96A1D66)_drop-shadow(0px_1px_24px_#FFA72C4D)]",
    threshold: 25_000,
  },
  silver: {
    label: "Silver",
    badgeColor: "green",
    tooltip: "Receive a Silver chest for every $100k volume.",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#80850680)_drop-shadow(0px_1px_24px_#B8BE0833)]",
    threshold: 100_000,
  },
  gold: {
    label: "Gold",
    badgeColor: "rice",
    tooltip: "Receive a Gold chest for every $250k volume.",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#E3BD6666)_drop-shadow(0px_1px_24px_#DCA54333)]",
    threshold: 250_000,
  },
  crystal: {
    label: "Crystal",
    badgeColor: "blue",
    tooltip: "Receive a Crystal chest for every $500k volume.",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#BCB8EB80)_drop-shadow(0px_1px_24px_#FFFFFF4D)]",
    threshold: 500_000,
  },
};

type BoxCardProps = {
  variant: BoxVariant;
  volume: number;
  className?: string;
  onClick?: () => void;
};

export const BoxCard: React.FC<BoxCardProps> = ({
  className,
  onClick,
  volume,
  variant,
}) => {
  const { isLg } = useMediaQuery();
  const { badgeColor, imageShadow, label, threshold } = VariantConfig[variant];

  const quantity = Math.max(Math.floor(volume / threshold), 0);
  const isLocked = quantity < 1;

  const handleClick = () => {
    if (isLocked) return;
    onClick?.();
    if (!onClick) {
      // Temporary action until a real handler is provided.
      // eslint-disable-next-line no-console
      console.log(`BoxCard clicked: ${variant}`);
    }
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
              <Tooltip
                className="max-w-[21rem]"
                content={VariantConfig[variant].tooltip}
                placement="top"
              >
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
              imageShadow,
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
      <Button
        size={isLg ? "md" : "sm"}
        className="px-8 lg:px-10"
        variant="primary"
        isDisabled={isLocked}
        onClick={handleClick}
      >
        Open
      </Button>
    </div>
  );
};
