import { useAccount } from "../../../../../../sdk/packages/dango/src/store/react";
import { BorrowAssetCard } from "./BorrowAssetCard";

export const MarginAssetsTable: React.FC = () => {
  const { account } = useAccount();

  return (
    <div className="bg-surface-rose-200 p-3 flex flex-col gap-4 rounded-3xl max-w-[40rem] w-full border border-brand-green flex-1">
      <div className="flex flex-col gap-1">
        <p className="uppercase text-center text-lg mb-3 text-typography-rose-500 font-extrabold">
          Assets
        </p>
        <div className="grid grid-cols-[1fr_100px_100px] px-4 text-xs font-extrabold text-typography-rose-500 tracking-widest uppercase min-h-8">
          <p className="self-end">Assets</p>
          <p className="self-end text-end">Deposited</p>
          <p className="w-full text-end">Borrow Capacity</p>
        </div>
        <BorrowAssetCard
          deposited={{ denom: "uusdc", amount: "10000" }}
          borrowed={{ denom: "uusdc", amount: "1000" }}
        />
      </div>
    </div>
  );
};
