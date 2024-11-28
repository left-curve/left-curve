import {
  AccountInfo,
  SafeMembersTable,
  SafePortfolioTable,
  SafeProposalsTable,
  Tab,
  Tabs,
} from "@dango/shared";

import type { Account, AccountType } from "@left-curve/types";

interface Props {
  account: Account;
}

export const ManageSafe: React.FC<Props> = ({ account }) => {
  return (
    <Tabs
      key="manage-safe-tabs"
      classNames={{ container: "min-h-full w-full flex-1", tabsWrapper: "mb-10" }}
    >
      <Tab key="assets" title="Assets">
        <div className="flex flex-col gap-4 justify-center items-center">
          <AccountInfo avatarUri="/images/safe.svg" account={account} />
          <SafePortfolioTable account={account} />
        </div>
      </Tab>
      <Tab key="proposals" title="Proposals">
        <SafeProposalsTable account={account} />
      </Tab>
      <Tab key="members" title="Members">
        <SafeMembersTable account={account as Account<typeof AccountType.Safe>} />
      </Tab>
    </Tabs>
  );
};
