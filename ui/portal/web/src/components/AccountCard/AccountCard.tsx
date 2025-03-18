import { type Account, AccountType } from "@left-curve/dango/types";
import type React from "react";

import { Badge, BorrowBar, IconCopy, TruncateText, twMerge } from "@left-curve/applets-kit";
import { AccountCardOptions } from "./AccountCardOptions";

interface Props {
  account: Account;
  balance: string;
  balanceChange?: string;
}

export const AccountCard: React.FC<Props> = ({ account, balance, balanceChange }) => {
  const { address, type } = account;
  const name = `${account?.type} #${account?.index}`;

  const { bgColor, badge, img, imgClassName, text } = AccountCardOptions[type];

  return (
    <div
      className={twMerge(
        "shadow-account-card w-full max-w-[20.5rem] lg:min-w-[20.5rem] h-[9.75rem] relative overflow-hidden rounded-md flex flex-col justify-between p-4",
        bgColor,
      )}
    >
      <img
        src={img}
        alt="account-card-dog"
        className={twMerge("absolute right-0 bottom-0 select-none drag-none", imgClassName)}
      />
      <div className="flex items-center justify-between relative z-10">
        <div className="flex gap-4 ">
          <div className="flex flex-col">
            <p className="exposure-m-italic capitalize">{name}</p>
            <div className="flex gap-1 items-center">
              <TruncateText
                text={address}
                className="diatype-xs-medium text-gray-500"
                start={4}
                end={4}
              />
              <IconCopy copyText={address} className="w-4 h-4 cursor-pointer text-gray-500" />
            </div>
          </div>
          <Badge text={text} color={badge} className="h-fit capitalize" size="s" />
        </div>
      </div>
      {type === AccountType.Margin ? (
        <BorrowBar borrow={0} borrowed={0} total={0} />
      ) : (
        <div className="flex gap-2 items-center relative z-10">
          <p className="h4-regular">{balance}</p>
          <p className="text-sm font-bold text-status-success">{balanceChange}</p>
        </div>
      )}
    </div>
  );
};
