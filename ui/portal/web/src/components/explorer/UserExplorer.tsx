import { Badge, Tabs, TruncateText, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  useExplorerUser,
  useExplorerUserTransactions,
  type AccountWithDetails,
  type ExplorerUserData,
} from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { createContext, useContext, useState } from "react";

import { AssetsTable } from "./AssetsTable";
import { HeaderExplorer } from "./HeaderExplorer";
import { TransactionsTable } from "./TransactionsTable";

import type { Address, IndexedTransaction } from "@left-curve/dango/types";
import type React from "react";
import type { PropsWithChildren } from "react";

type UserExplorerContextType = {
  username: string;
  userData: ExplorerUserData | null;
  transactions: IndexedTransaction[];
  transactionsPagination: {
    isLoading: boolean;
    goNext: () => void;
    goPrev: () => void;
    hasNextPage: boolean;
    hasPreviousPage: boolean;
  };
  isLoading: boolean;
  isNotFound: boolean;
};

const UserExplorerContext = createContext<UserExplorerContextType | null>(null);

const useUserExplorer = () => {
  const context = useContext(UserExplorerContext);
  if (!context) {
    throw new Error("useUserExplorer must be used within UserExplorerContext");
  }
  return context;
};

type UserExplorerProps = {
  username: string;
};

const Root: React.FC<PropsWithChildren<UserExplorerProps>> = ({ username, children }) => {
  const { data: userData, isLoading: isUserLoading, isNotFound } = useExplorerUser(username);

  const accountAddresses = userData?.accounts.map((a) => a.address) || [];
  const {
    data: transactions,
    pagination: transactionsPagination,
    isLoading: isTransactionsLoading,
  } = useExplorerUserTransactions(accountAddresses as Address[]);

  const isLoading = isUserLoading || isTransactionsLoading;

  return (
    <UserExplorerContext.Provider
      value={{
        username,
        userData,
        transactions,
        transactionsPagination,
        isLoading,
        isNotFound,
      }}
    >
      <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">{children}</div>
    </UserExplorerContext.Provider>
  );
};

const StatItem: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="flex flex-col">
    <p className="diatype-m-medium text-ink-tertiary-500">{label}</p>
    <p className="diatype-mono-medium text-ink-secondary-700">{value}</p>
  </div>
);

const Header: React.FC = () => {
  const { userData, isLoading } = useUserExplorer();

  if (isLoading || !userData) return null;

  return (
    <div className="flex flex-col md:flex-row gap-4">
      <div className="flex items-start gap-4 rounded-xl p-4 bg-surface-secondary-rice shadow-account-card min-h-[10rem] md:min-w-[21.7rem]">
        <img src="/images/avatar.png" alt="avatar" className="w-16 h-16 rounded-lg object-cover" />
        <div className="flex flex-col gap-1">
          <p className="h4-bold text-ink-primary-900">{userData.user.name}</p>
        </div>
      </div>

      <div className="flex-1 flex flex-col justify-between gap-4 rounded-xl p-4 bg-surface-secondary-rice shadow-account-card min-h-[10rem]">
        <p className="h3-bold text-ink-primary-900">{userData.totalValue}</p>
        <div className="flex flex-wrap gap-6 justify-between">
          <StatItem label={m["explorer.user.stats.totalAssets"]()} value={userData.totalValue} />
          <StatItem
            label={m["explorer.user.stats.totalAccounts"]()}
            value={String(userData.totalAccounts)}
          />
        </div>
      </div>
    </div>
  );
};

type StackedAccountCardProps = {
  account: AccountWithDetails;
  isFirst: boolean;
  href: string;
  onClick: () => void;
};

const StackedAccountCard: React.FC<StackedAccountCardProps> = ({
  account,
  isFirst,
  href,
  onClick,
}) => {
  return (
    <a
      href={href}
      className={twMerge(
        "shadow-account-card w-full max-w-[22.5rem] md:max-w-[20.5rem] flex-shrink-0 h-[10rem] relative overflow-hidden rounded-xl flex flex-col justify-between p-4 cursor-pointer",
        "bg-account-card-red text-ink-secondary-700",
        "transition-all duration-200 ease-out hover:-translate-y-2 hover:z-50",
        !isFirst && "-mt-[4rem]",
      )}
      onClick={(e) => {
        e.preventDefault();
        onClick();
      }}
    >
      <div className="flex items-start justify-between relative z-10">
        <div className="flex flex-col">
          <p className="exposure-m-italic capitalize text-ink-tertiary-500">
            {m["explorer.user.accountNumber"]({ index: account.index })}
          </p>
          <TruncateText
            text={account.address}
            className="diatype-xs-medium text-ink-tertiary-500"
            start={4}
            end={4}
          />
        </div>
        <div className="flex flex-col gap-1 items-end">
          <p className="diatype-m-bold text-ink-tertiary-500">{account.balanceUSD}</p>
          <Badge
            text={account.isActive ? m["explorer.user.active"]() : m["explorer.user.inactive"]()}
            color={account.isActive ? "blue" : "gray"}
            className="h-fit capitalize"
            size="s"
          />
        </div>
      </div>
    </a>
  );
};

const AccountsStack: React.FC = () => {
  const { userData } = useUserExplorer();
  const navigate = useNavigate();

  if (!userData) return null;

  return (
    <div className="flex flex-col">
      <h4 className="h4-bold text-ink-primary-900 mb-[2.22rem]">{m["explorer.user.accounts"]()}</h4>
      <div className="flex flex-col">
        {userData.accounts.map((account, index) => (
          <StackedAccountCard
            key={account.address}
            account={account}
            isFirst={index === 0}
            href={`/account/${account.address}`}
            onClick={() => navigate({ to: `/account/${account.address}` })}
          />
        ))}
      </div>
    </div>
  );
};

const Content: React.FC = () => {
  const { userData, transactions, transactionsPagination, isLoading } = useUserExplorer();
  const tabAssets = m["explorer.user.tabs.assets"]();
  const tabTransactions = m["explorer.user.tabs.transactions"]();
  const [activeTab, setActiveTab] = useState<string>(tabAssets);

  if (isLoading || !userData) return null;

  return (
    <div className="flex flex-col lg:flex-row gap-6 rounded-xl p-4 bg-surface-secondary-rice shadow-account-card">
      <div className="w-full max-w-[22.5rem] md:max-w-[20.5rem] flex-shrink-0">
        <AccountsStack />
      </div>

      <div className="flex-1 flex flex-col gap-4 min-w-0">
        <Tabs
          layoutId="user-explorer-tabs"
          selectedTab={activeTab}
          onTabChange={(tab) => setActiveTab(tab)}
          keys={[tabAssets, tabTransactions]}
        />

        <div className="max-w-full overflow-x-auto scrollbar-none">
          {activeTab === tabAssets && (
            <AssetsTable
              balances={userData.aggregatedBalances}
              classNames={{ base: "p-0 shadow-none bg-transparent" }}
            />
          )}
          {activeTab === tabTransactions && (
            <TransactionsTable
              transactions={transactions}
              pagination={transactionsPagination}
              classNames={{ base: "p-0 shadow-none bg-transparent" }}
            />
          )}
        </div>
      </div>
    </div>
  );
};

const NotFound: React.FC = () => {
  const { username, isNotFound, isLoading } = useUserExplorer();

  if (isLoading || !isNotFound) return null;

  return (
    <div className="w-full md:max-w-[76rem] p-4">
      <HeaderExplorer>
        <div className="flex flex-col gap-2 items-center">
          <h3 className="exposure-m-italic text-ink-secondary-700">
            {m["explorer.user.notFound.title"]()}
          </h3>
          <p className="diatype-m-medium max-w-[42.5rem] text-center text-ink-tertiary-500">
            {m["explorer.user.notFound.description"]({ username })}
          </p>
        </div>
      </HeaderExplorer>
    </div>
  );
};

export const UserExplorer = Object.assign(Root, {
  Header,
  Content,
  NotFound,
});
