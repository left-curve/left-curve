import {
  AccountInfo,
  SafeMembersTable,
  SafePortfolioTable,
  SafeProposalsTable,
  Tab,
  Tabs,
} from "@dango/shared";
import { useAccount } from "@leftcurve/react";

import type { AccountType } from "@leftcurve/types";

export const ManageSafe: React.FC = () => {
  const { account } = useAccount<typeof AccountType.Safe>();
  if (!account) return null;
  return (
    <Tabs
      key="manage-safe-tabs"
      classNames={{ container: "min-h-full w-full flex-1", tabsWrapper: "mb-10" }}
    >
      <Tab key="assets" title="Assets">
        <div className="flex flex-col gap-4 justify-center items-center">
          <AccountInfo avatarUri="/images/safe.svg" />
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
