import { MarginAccountInfo, MarginAssetsTable, MarginDebtsTable } from "@left-curve/applets-kit";

import type { Account } from "@left-curve/types";

interface Props {
  account: Account;
}

const ManageMargin: React.FC<Props> = ({ account }) => {
  return (
    <div className="flex flex-1 flex-col w-full items-center gap-14 mt-14">
      <MarginAccountInfo avatarUrl="/images/avatars/margin.svg" account={account} />
      <div className="flex w-full gap-4 flex-col lg:flex-row items-center justify-center">
        <MarginAssetsTable />
        <MarginDebtsTable />
      </div>
    </div>
  );
};

export default ManageMargin;
