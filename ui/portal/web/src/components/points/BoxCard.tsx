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

type BoxVariant = "bronze" | "silver" | "gold" | "crystal";

const VariantConfig: Record<
  BoxVariant,
  {
    label: string;
    badgeColor: "red" | "green" | "rice" | "blue";
    tooltip: string;
    imageShadow: string;
  }
> = {
  bronze: {
    label: "Bronze",
    badgeColor: "red",
    tooltip: "Receive a Bronze chest for every $25k volume.",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#C96A1D66)_drop-shadow(0px_1px_24px_#FFA72C4D)]",
  },
  silver: {
    label: "Silver",
    badgeColor: "green",
    tooltip: "Receive a Silver chest for every $100k volume.",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#80850680)_drop-shadow(0px_1px_24px_#B8BE0833)]",
  },
  gold: {
    label: "Gold",
    badgeColor: "rice",
    tooltip: "Receive a Gold chest for every $250k volume.",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#E3BD6666)_drop-shadow(0px_1px_24px_#DCA54333)]",
  },
  crystal: {
    label: "Crystal",
    badgeColor: "blue",
    tooltip: "Receive a Crystal chest for every $500k volume.",
    imageShadow:
      "[filter:drop-shadow(0px_4px_100px_#BCB8EB80)_drop-shadow(0px_1px_24px_#FFFFFF4D)]",
  },
};

type BoxCardProps = {
  variant: BoxVariant;
  quantity?: number;
  lock?: boolean;
  className?: string;
  onClick?: () => void;
};

export const BoxCard: React.FC<BoxCardProps> = ({
  className,
  lock = false,
  onClick,
  quantity = 1,
  variant,
}) => {
  const { isLg } = useMediaQuery();
  const { badgeColor, imageShadow, label } = VariantConfig[variant];

  const handleClick = () => {
    if (lock) return;
    onClick?.();
    if (!onClick) {
      // Temporary action until a real handler is provided.
      // eslint-disable-next-line no-console
      console.log(`BoxCard clicked: ${variant}`);
    }
  };

  return (
    <div className={twMerge("flex flex-col items-center gap-3 lg:gap-4", className)}>
      <div className="relative w-[9.6rem] h-[9.6rem] lg:w-[11.6rem] lg:h-[11.9rem] rounded-xl border border-outline-primary-gray bg-surface-secondary-rice shadow-account-card">
        <Badge
          size="m"
          color={badgeColor}
          text={
            <div className="flex pl-1 items-center">
              {label}
              <Tooltip
                className="max-w-[21rem]"
                content={VariantConfig[variant].tooltip}
                placement="top"
              >
                <IconInfo className="inline-block ml-1 w-4 h-4" />
              </Tooltip>
            </div>
          }
          className="absolute top-2 left-2 z-10 capitalize rounded-full"
        />
        <img
          src={`/images/points/boxes/${variant}.png`}
          alt={`${label} chest`}
          className={twMerge(
            "w-full h-full object-contain select-none drag-none absolute inset-0 -top-4 lg:-top-6",
            imageShadow,
          )}
        />
        {!lock ? (
          <p className="absolute bottom-3 left-1/2 -translate-x-1/2 diatype-lg-bold text-ink-primary-900 z-10">
            x{quantity}
          </p>
        ) : null}
        {lock ? (
          <div className="flex items-center justify-center rounded-full bg-surface-tertiary-gray absolute bottom-3 right-3 w-8 h-8 z-10">
            <IconLock className=" w-6 h-6 text-utility-warning-600" />
          </div>
        ) : null}
      </div>
      <Button
        size={isLg ? "md" : "sm"}
        className="px-8 lg:px-10"
        variant="primary"
        isDisabled={lock}
        onClick={handleClick}
      >
        Open
      </Button>
    </div>
  );
};
