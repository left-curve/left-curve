import { useAccount, useBalances } from "@leftcurve/react";
import { Button } from "~/components";
import { AssetCard } from "./AssetCard";

export const SpotPortfolio: React.FC = () => {
  const { account } = useAccount();
  const { data: balances = {} } = useBalances({ address: account!.address });

  return (
    <div className="bg-sand-50 p-4 flex flex-col gap-4 rounded-3xl max-w-[40rem] w-full">
      <div className="flex flex-col gap-3 sm:flex-row w-full">
        <Button color="danger" className="flex-1 min-h-11 italic rounded-3xl">
          Send
        </Button>
        <Button color="danger" className="flex-1 min-h-11 italic rounded-3xl">
          Receive
        </Button>
      </div>
      <div className="flex flex-col gap-1">
        <div className="grid grid-cols-[1fr_100px_100px] px-2 text-sm font-extrabold text-sand-800/50 font-diatype-rounded mx-2 tracking-widest uppercase">
          <p>Assets</p>
          <p>Deposited</p>
          <p className="w-full text-end">Amount</p>
        </div>
        {Object.entries(balances).map(([denom, amount]) => (
          <AssetCard key={denom} coin={{ denom, amount }} />
        ))}
      </div>
    </div>
  );
};
