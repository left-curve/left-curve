import { type Account, AccountType } from "@left-curve/dango/types";
import type React from "react";
import { Badge } from "./Badge";
import TruncateText from "./TruncateText";

interface Props {
  account: Account;
  balance: string;
  balanceChange: string;
}

const accountTypeBadgeColor = {
  [AccountType.Spot]: "blue",
  [AccountType.Margin]: "blue",
  [AccountType.Safe]: "green",
} as const;

export const AccountCard: React.FC<Props> = ({ account, balance, balanceChange }) => {
  if (!account) return null;
  const { address, type } = account;
  const name = `${account?.type} #${account?.index}`;

  return (
    <div className="shadow-account-card w-full max-w-[20.5rem] h-[9.75rem] bg-account-card-red relative overflow-hidden rounded-md flex flex-col justify-between p-4">
      <img
        src="/images/account-card/dog.svg"
        alt="account-card-dog"
        className="absolute right-0 bottom-0"
      />
      <div className="flex gap-4">
        <div className="flex flex-col">
          <p className="exposure-m-italic capitalize">{name}</p>
          <TruncateText
            text={address}
            className="diatype-xs-medium text-gray-500"
            start={4}
            end={4}
          />
        </div>
        <Badge text={type} color="blue" className="h-fit capitalize" />
      </div>
      <div className="flex gap-2 items-center">
        <p className="h4-regular">{balance}</p>
        <p className="text-sm font-bold text-[#25B12A]">{balanceChange}</p>
      </div>
    </div>
  );
};
