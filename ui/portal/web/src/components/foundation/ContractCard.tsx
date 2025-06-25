import { AddressVisualizer, Badge, TextCopy, TruncateText, twMerge } from "@left-curve/applets-kit";

import type { Address } from "@left-curve/dango/types";
import type React from "react";

type ContractCardProps = {
  address: Address;
  balance: string;
  balanceChange?: string;
  isSelectorActive?: boolean;
  onTriggerAction?: () => void;
};

export const ContractCard: React.FC<ContractCardProps> = ({ address, balance, balanceChange }) => {
  return (
    <div
      className={twMerge(
        "shadow-account-card w-full max-w-[22.5rem] md:max-w-[20.5rem] lg:min-w-[20.5rem] h-[10rem] relative overflow-hidden rounded-xl flex flex-col justify-between p-4",
        "bg-account-card-contract",
      )}
    >
      <img
        src="/images/emojis/detailed/factory.svg"
        alt="factory"
        className="absolute right-0 bottom-2 select-none drag-none h-[141px] opacity-50"
      />
      <div className="flex flex-col relative z-10">
        <div className="flex gap-1 items-center">
          <AddressVisualizer
            address={address}
            classNames={{ text: "exposure-m-italic capitalize" }}
          />
          <Badge text="App" color="green" className="h-fit capitalize" size="s" />
        </div>
        <div className="flex gap-1 items-center">
          <TruncateText
            text={address}
            className="diatype-xs-medium text-gray-500"
            start={4}
            end={4}
          />
          <TextCopy copyText={address} className="w-4 h-4 cursor-pointer text-gray-500" />
        </div>
      </div>

      <div className="flex gap-2 items-center relative z-10">
        <p className="h4-regular">{balance}</p>
        {/*  <p className="text-sm font-bold text-status-success">{balanceChange}</p> */}
      </div>
    </div>
  );
};
