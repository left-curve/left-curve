import { useBalances } from "@leftcurve/react";
import { AssetCard } from "./AssetCard";

import type { Account } from "@leftcurve/types";

interface Props {
  account: Account;
  topComponent?: React.ReactNode;
  bottomComponent?: React.ReactNode;
}

export const PortfolioTable: React.FC<Props> = ({ topComponent, bottomComponent, account }) => {
  const { data: balances = {} } = useBalances({ address: account?.address });

  return (
    <div className="bg-sand-50 p-4 flex flex-col gap-4 rounded-3xl max-w-[40rem] w-full">
      {topComponent}

      <div className="flex flex-col gap-1">
        <div className="grid grid-cols-[1fr_100px_100px] px-2 text-sm font-extrabold text-sand-800/50 font-diatype-rounded mx-2 tracking-widest uppercase">
          <p>Assets</p>
          <p>Quantity</p>
          <p className="w-full text-end">Value</p>
        </div>
        {Object.entries(balances).map(([denom, amount]) => (
          <AssetCard key={denom} coin={{ denom, amount }} />
        ))}
      </div>
      {bottomComponent}
    </div>
  );
};
