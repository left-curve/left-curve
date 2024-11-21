import { MarginAccountInfo, MarginAssetsTable, MarginDebtsTable } from "@dango/shared";

export const ManageMargin: React.FC = () => {
  return (
    <div className="flex flex-1 flex-col w-full items-center gap-14 mt-14">
      <MarginAccountInfo avatarUrl="/images/avatars/margin.svg" />
      <div className="flex w-full gap-4 flex-col lg:flex-row items-center justify-center">
        <MarginAssetsTable />
        <MarginDebtsTable />
      </div>
    </div>
  );
};
