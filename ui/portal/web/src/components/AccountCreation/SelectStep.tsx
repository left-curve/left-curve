import { AccountDescriptionCard, useWizard } from "@left-curve/applets-kit";
import { motion } from "framer-motion";
import type React from "react";

import { AccountType, type AccountTypes } from "@left-curve/dango/types";

export const SelectStep: React.FC = () => {
  const { setData, goToStep } = useWizard();

  const handleSelect = (accountType: AccountTypes) => {
    setData({ accountType });
    goToStep(1);
  };

  return (
    <motion.div
      className="flex flex-col w-full justify-center gap-8"
      initial={{ translateY: -100 }}
      animate={{ translateY: 0 }}
      exit={{ translateY: 100 }}
    >
      <div className="flex flex-col gap-8 items-center text-center">
        <h3 className="font-bold text-typography-black-200 font-diatype-rounded tracking-widest uppercase">
          Select your account type
        </h3>
        <p className="text-typography-black-100 max-w-[430px]">
          Portal allows user to create different types of accounts to interact within the entire
          ecosystem
        </p>
      </div>
      <div className="flex flex-col gap-2">
        <AccountDescriptionCard
          title="Spot account"
          img="/images/avatars/spot.svg"
          description="Can hold any asset and partake in any activity; cheapest gas cost; can only take over-collateralized loans."
          className="bg-surface-rose-100 hover:bg-surface-rose-200"
          onClick={() => handleSelect(AccountType.Spot)}
        />
        <AccountDescriptionCard
          title="Margin account"
          img="/images/avatars/margin.svg"
          className="bg-surface-purple-100 hover:bg-surface-purple-200"
          description="Can take under-collateralized loans up to 5x leverage; can only hold whitelisted assets and partake in whitelisted activity"
          onClick={() => handleSelect(AccountType.Margin)}
        />
      </div>
    </motion.div>
  );
};
