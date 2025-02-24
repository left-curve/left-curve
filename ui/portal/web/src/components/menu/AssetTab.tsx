import { AssetCard } from "@left-curve/applets-kit";
import { useAccount, useBalances } from "@left-curve/store-react";

import type React from "react";

export const AssetTab: React.FC = () => {
  const { account } = useAccount();

  if (!account) return null;

  const { data: balances = {} } = useBalances({ address: account.address });

  return (
    <div className="flex flex-col w-full overflow-y-auto scrollbar-none pb-4">
      {Object.entries(balances).map(([denom, amount]) => (
        <AssetCard key={denom} coin={{ amount, denom }} />
      ))}
    </div>
  );
};
