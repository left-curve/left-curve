import { Button, IconFlash, IconTimer, useMediaQuery } from "@left-curve/applets-kit";
import type React from "react";
import { OATCard } from "./OATCard";

type OATsSectionProps = {
  pointsBoost?: number;
  endsIn?: string;
  onLinkWallet?: () => void;
};

export const OATsSection: React.FC<OATsSectionProps> = ({
  pointsBoost = 100,
  endsIn = "2 days 21:24:32",
  onLinkWallet,
}) => {
  const { isLg } = useMediaQuery();

  return (
    <div className="flex flex-col gap-4">
      <p className="h3-bold text-ink-primary-900">My OATs</p>

      <div className="flex flex-col lg:flex-row gap-3 md:gap-6">
        <div className="flex items-center justify-between gap-3 px-4 py-2 bg-surface-tertiary-gray shadow-account-card rounded-full flex-1">
          <IconFlash className="w-6 h-6 text-primitives-green-light-400" />
          <p className="diatype-m-regular text-ink-primary-900">
            <span className="diatype-m-bold">{pointsBoost}%</span> Points Boost
          </p>
        </div>
        <div className="flex items-center justify-between gap-3 px-4 py-2 bg-surface-tertiary-gray shadow-account-card rounded-full flex-1">
          <IconTimer className="w-6 h-6 text-brand-red-bean" />
          <p className="diatype-m-regular text-ink-primary-900">
            Ends in <span className="diatype-m-bold">{endsIn}</span>
          </p>
        </div>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
        <OATCard type="hurrah" isLocked={false} />
        <OATCard type="trader" isLocked={false} />
        <OATCard type="wizard" isLocked={true} />
        <OATCard type="supporter" isLocked={true} />
      </div>

      <Button
        size={isLg ? "md" : "lg"}
        variant="primary"
        onClick={onLinkWallet}
        className="w-full lg:w-fit"
      >
        Link Your EVM Wallet
      </Button>
    </div>
  );
};
