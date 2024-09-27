"use client";

import {
  AccountInfo,
  SafeMembersTable,
  SafePortfolioTable,
  SafeProposalsTable,
  Tab,
  Tabs,
} from "@leftcurve/dango";
import { useAccount } from "@leftcurve/react";
import type { AccountType } from "@leftcurve/types";

export const ManageSafe: React.FC = () => {
  const { account } = useAccount<typeof AccountType.Safe>();
  if (!account) return null;
  return (
    <Tabs
      key="manage-safe-tabs"
      className="min-h-full w-full flex-1"
      classNames={{ tabsContainer: "mb-10", contentContainer: "min-h-[60vh]" }}
    >
      <Tab key="assets" title="Assets">
        <div className="flex flex-col gap-4 justify-center items-center">
          <AccountInfo avatarUri="/images/safe-avatar.png" />
          <SafePortfolioTable />
        </div>
      </Tab>
      <Tab key="proposals" title="Proposals">
        <SafeProposalsTable account={account} />
      </Tab>
      <Tab key="members" title="Members">
        <SafeMembersTable account={account} />
      </Tab>
    </Tabs>
  );
};
