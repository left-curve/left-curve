import { AccountCardPreview } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";

import type React from "react";

export const AccountTab: React.FC = () => {
  const { account, accounts, changeAccount } = useAccount();

  if (!account) return null;

  return (
    <div className="flex flex-col w-full overflow-y-auto gap-4 scrollbar-none p-4 pb-[7rem] relative">
      {accounts
        ?.filter((acc) => acc.address !== account.address)
        .map((account) => (
          <AccountCardPreview
            key={account.address}
            account={account}
            onAccountSelect={(acc) => changeAccount?.(acc)}
          />
        ))}
    </div>
  );
};
