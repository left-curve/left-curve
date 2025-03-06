import { useBalances, usePrices } from "@left-curve/store-react";
import { twMerge } from "../../../utils";

import type { Account, AccountTypes } from "@left-curve/dango/types";
import { Badge } from "../Badge";
import TruncateText from "../TruncateText";
import { AccountCardOptions } from "./AccountCardOptions";

export const AccountCardPreview: React.FC<{
  account: Account;
  onAccountSelect: (account: Account) => void;
}> = ({ account, onAccountSelect }) => {
  const { address } = account;

  const type = account?.type as AccountTypes;
  const name = `${type} #${account?.index}`;

  const { bgColor, badge, text } = AccountCardOptions[type];

  const { data: balances = {} } = useBalances({ address });
  const { calculateBalance } = usePrices();

  const totalBalance = calculateBalance(balances, { format: true });
  return (
    <div
      className={twMerge(
        "shadow-account-card w-full max-w-[20.5rem] lg:min-w-[20.5rem] h-[9.75rem] relative overflow-hidden rounded-md flex flex-col justify-between p-4 cursor-pointer",
        "mb-[-6.5rem]",
        bgColor,
      )}
      onClick={() => onAccountSelect(account)}
    >
      <div className="flex items-center justify-between relative z-10">
        <div className="flex gap-4 ">
          <div className="flex flex-col">
            <p className="exposure-m-italic capitalize text-gray-400">{name}</p>
            <div className="flex gap-1 items-center">
              <TruncateText
                text={address}
                className="diatype-xs-medium text-gray-500"
                start={4}
                end={4}
              />
              {/* <IconCopy copyText={address} className="w-4 h-4 cursor-pointer text-gray-500" /> */}
            </div>
          </div>
        </div>
        <div>
          <p className="diatype-m-bold text-gray-400">{totalBalance}</p>
          <Badge text={text} color={badge} className="h-fit capitalize" size="s" />
        </div>
      </div>
    </div>
  );
};
