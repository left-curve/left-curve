"use client";

import {
  MarginAccountInfo,
  MarginAssetsTable,
  MarginDebtsTable,
} from "../../../packages/ui/build/index.mjs";

export const ManageMargin: React.FC = () => {
  return (
    <div className="flex flex-1 flex-col w-full gap-12 items-center">
      <MarginAccountInfo avatarUrl="/images/avatars/margin.png" />
      <div className="flex w-full gap-4 flex-col lg:flex-row items-center justify-start">
        <MarginAssetsTable />
        <MarginDebtsTable />
      </div>
    </div>
  );
};
