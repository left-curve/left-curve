import { BorrowAssetCard } from "./BorrowAssetCard";

export const MarginDebtsTable: React.FC = () => {
  return (
    <div className="bg-surface-rose-200 p-3 flex flex-col gap-4 rounded-3xl max-w-[40rem] w-full border border-brand-pink flex-1">
      <div className="flex flex-col gap-1">
        <p className="uppercase text-center text-lg mb-3 text-typography-rose-500 font-extrabold">
          Debts
        </p>
        <div className="flex justify-between items-end px-4 text-xs font-extrabold text-typography-rose-500 tracking-widest uppercase min-h-8">
          <p>Assets</p>
          <p className="w-full text-end">Borrowed</p>
        </div>
        <BorrowAssetCard borrowed={{ amount: "1000", denom: "uusdc" }} />
      </div>
    </div>
  );
};
