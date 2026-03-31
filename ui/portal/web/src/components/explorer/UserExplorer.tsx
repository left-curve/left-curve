import { Badge, Tab, Tabs, TruncateText, twMerge } from "@left-curve/applets-kit";
import { useNavigate } from "@tanstack/react-router";
import { createContext, useContext, useState } from "react";

import { AssetsTable } from "./AssetsTable";
import { HeaderExplorer } from "./HeaderExplorer";
import { TransactionsTable } from "./TransactionsTable";

import type { Coins, IndexedTransaction } from "@left-curve/dango/types";
import type React from "react";
import type { PropsWithChildren } from "react";

// ============================================================================
// Mock Data Types
// ============================================================================

type MockUserProfile = {
  username: string;
  avatar: string;
  badge: string;
  totalValue: string;
  totalDebt: string;
  totalAssets: string;
  totalAccounts: number;
  dateJoined: string;
};

type MockAccount = {
  address: string;
  index: number;
  balance: string;
};

// ============================================================================
// Mock Data
// ============================================================================

const mockUserProfile: MockUserProfile = {
  username: "larry.user",
  avatar: "/images/avatar.png",
  badge: "Left Curve Trader",
  totalValue: "$4,016",
  totalDebt: "$100.00",
  totalAssets: "$100.00",
  totalAccounts: 12,
  dateJoined: "10/09/2024, 12:08:03",
};

const mockAccounts: MockAccount[] = [
  { address: "0x1234567890abcdef1234567890abcdef12345678", index: 132548, balance: "$125.04M" },
  { address: "0x2345678901abcdef2345678901abcdef23456789", index: 132549, balance: "$125.04M" },
  { address: "0x3456789012abcdef3456789012abcdef34567890", index: 132550, balance: "$125.04M" },
  { address: "0x4567890123abcdef4567890123abcdef45678901", index: 132551, balance: "$125.04M" },
  { address: "0x5678901234abcdef5678901234abcdef56789012", index: 132552, balance: "$125.04M" },
  { address: "0x6789012345abcdef6789012345abcdef67890123", index: 132553, balance: "$125.04M" },
];

const mockBalances: Coins = {
  "factory/dango/usdc": "100000000000",
  "factory/dango/eth": "50000000000",
  dango: "25000000000",
};

const mockTransactions: IndexedTransaction[] = [
  {
    blockHeight: 29986907,
    createdAt: new Date(Date.now() - 1000 * 60 * 5).toISOString(),
    transactionType: "TX",
    transactionIdx: 0,
    sender: "0xB82C04...8B15CF" as `0x${string}`,
    hash: "0xB82C041234567890abcdef1234567890abcdef1234567890abcdef12348B15CF",
    hasSucceeded: true,
    errorMessage: "",
    gasWanted: 100000,
    gasUsed: 80000,
    messages: [
      {
        methodName: "Contract: Update Pric...",
        blockHeight: 29986907,
        contractAddr: "0x123456" as `0x${string}`,
        senderAddr: "0xB82C04" as `0x${string}`,
        orderIdx: 0,
        createdAt: new Date().toISOString(),
        data: {},
      },
    ],
    nestedEvents: "",
  },
  {
    blockHeight: 29986906,
    createdAt: new Date(Date.now() - 1000 * 60 * 10).toISOString(),
    transactionType: "TX",
    transactionIdx: 0,
    sender: "0xB82C04...8B15CF" as `0x${string}`,
    hash: "0xB82C04abcdef1234567890abcdef1234567890abcdef1234567890ab8B15CF",
    hasSucceeded: true,
    errorMessage: "",
    gasWanted: 100000,
    gasUsed: 75000,
    messages: [
      {
        methodName: "Contract: Update Pric...",
        blockHeight: 29986906,
        contractAddr: "0x123456" as `0x${string}`,
        senderAddr: "0xB82C04" as `0x${string}`,
        orderIdx: 0,
        createdAt: new Date().toISOString(),
        data: {},
      },
    ],
    nestedEvents: "",
  },
  {
    blockHeight: 29986905,
    createdAt: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
    transactionType: "TX",
    transactionIdx: 0,
    sender: "0xB82C04...8B15CF" as `0x${string}`,
    hash: "0xB82C04def1234567890abcdef1234567890abcdef1234567890abcde8B15CF",
    hasSucceeded: true,
    errorMessage: "",
    gasWanted: 100000,
    gasUsed: 85000,
    messages: [
      {
        methodName: "Contract: Update Pric...",
        blockHeight: 29986905,
        contractAddr: "0x123456" as `0x${string}`,
        senderAddr: "0xB82C04" as `0x${string}`,
        orderIdx: 0,
        createdAt: new Date().toISOString(),
        data: {},
      },
    ],
    nestedEvents: "",
  },
];

// ============================================================================
// Context
// ============================================================================

type UserExplorerContextType = {
  username: string;
  profile: MockUserProfile | null;
  accounts: MockAccount[];
  balances: Coins;
  transactions: IndexedTransaction[];
  isLoading: boolean;
};

const UserExplorerContext = createContext<UserExplorerContextType | null>(null);

const useUserExplorer = () => {
  const context = useContext(UserExplorerContext);
  if (!context) {
    throw new Error("useUserExplorer must be used within UserExplorerContext");
  }
  return context;
};

// ============================================================================
// Root Container
// ============================================================================

type UserExplorerProps = {
  username: string;
};

const Root: React.FC<PropsWithChildren<UserExplorerProps>> = ({ username, children }) => {
  // In real implementation, this would fetch user data from API
  const profile = mockUserProfile;
  const accounts = mockAccounts;
  const balances = mockBalances;
  const transactions = mockTransactions;

  return (
    <UserExplorerContext.Provider
      value={{
        username,
        profile,
        accounts,
        balances,
        transactions,
        isLoading: false,
      }}
    >
      <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">{children}</div>
    </UserExplorerContext.Provider>
  );
};

// ============================================================================
// Header Component
// ============================================================================

const StatItem: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="flex flex-col">
    <p className="diatype-m-medium text-ink-tertiary-500">{label}</p>
    <p className="diatype-mono-medium text-ink-secondary-700">{value}</p>
  </div>
);

const Header: React.FC = () => {
  const { profile, isLoading } = useUserExplorer();

  if (isLoading || !profile) return null;

  return (
    <div className="flex flex-col md:flex-row gap-4">
      {/* Left Container: Avatar + Username + Badge */}
      <div className="flex items-start gap-4 rounded-xl p-4 bg-surface-secondary-rice shadow-account-card min-h-[10rem] md:min-w-[21.7rem]">
        <img src={profile.avatar} alt="avatar" className="w-16 h-16 rounded-lg object-cover" />
        <div className="flex flex-col gap-1">
          <p className="h4-bold text-ink-primary-900">{profile.username}</p>
          <Badge text={profile.badge} color="rice" size="s" />
        </div>
      </div>

      {/* Right Container: Total Value + Stats */}
      <div className="flex-1 flex flex-col justify-between gap-4 rounded-xl p-4 bg-surface-secondary-rice shadow-account-card min-h-[10rem]">
        {/* Total Value - top */}
        <p className="h3-bold text-ink-primary-900">{profile.totalValue}</p>

        {/* Stats row - bottom */}
        <div className="flex flex-wrap gap-6 justify-between">
          <StatItem label="Total Debt" value={profile.totalDebt} />
          <StatItem label="Total Assets" value={profile.totalAssets} />
          <StatItem label="Total Accounts" value={String(profile.totalAccounts)} />
          <StatItem label="Date Joined" value={profile.dateJoined} />
        </div>
      </div>
    </div>
  );
};

// ============================================================================
// Stacked Account Card
// ============================================================================

type StackedAccountCardProps = {
  account: MockAccount;
  isFirst: boolean;
  onClick: () => void;
};

const StackedAccountCard: React.FC<StackedAccountCardProps> = ({ account, isFirst, onClick }) => {
  return (
    <div
      className={twMerge(
        "shadow-account-card w-full max-w-[22.5rem] md:max-w-[20.5rem] flex-shrink-0 h-[10rem] relative overflow-hidden rounded-xl flex flex-col justify-between p-4 cursor-pointer",
        "bg-account-card-red text-ink-secondary-700",
        "transition-all duration-200 ease-out hover:-translate-y-2 hover:z-50",
        !isFirst && "-mt-[4rem]",
      )}
      onClick={onClick}
    >
      <div className="flex items-start justify-between relative z-10">
        <div className="flex flex-col">
          <p className="exposure-m-italic capitalize text-ink-tertiary-500">
            Account #{account.index}
          </p>
          <TruncateText
            text={account.address}
            className="diatype-xs-medium text-ink-tertiary-500"
            start={4}
            end={4}
          />
        </div>
        <div className="flex flex-col gap-1 items-end">
          <p className="diatype-m-bold text-ink-tertiary-500">{account.balance}</p>
          <Badge text="Active" color="blue" className="h-fit capitalize" size="s" />
        </div>
      </div>
    </div>
  );
};

// ============================================================================
// Accounts Stack
// ============================================================================

const AccountsStack: React.FC = () => {
  const { accounts } = useUserExplorer();
  const navigate = useNavigate();

  return (
    <div className="flex flex-col">
      <h4 className="h4-bold text-ink-primary-900 mb-[2.22rem]">Accounts</h4>
      <div className="flex flex-col">
        {accounts.map((account, index) => (
          <StackedAccountCard
            key={account.address}
            account={account}
            isFirst={index === 0}
            onClick={() => navigate({ to: `/account/${account.address}` })}
          />
        ))}
      </div>
    </div>
  );
};

// ============================================================================
// Content Component (Two-column layout)
// ============================================================================

const Content: React.FC = () => {
  const { profile, balances, transactions, isLoading } = useUserExplorer();
  const [activeTab, setActiveTab] = useState<string>("Assets");

  if (isLoading || !profile) return null;

  return (
    <div className="flex flex-col lg:flex-row gap-6 rounded-xl p-4 bg-surface-secondary-rice shadow-account-card">
      {/* LEFT: Stacked Account Cards */}
      <div className="w-full max-w-[22.5rem] md:max-w-[20.5rem] flex-shrink-0">
        <AccountsStack />
      </div>

      {/* RIGHT: Tabs Content */}
      <div className="flex-1 flex flex-col gap-4 min-w-0">
        <Tabs
          layoutId="user-explorer-tabs"
          selectedTab={activeTab}
          onTabChange={(tab) => setActiveTab(tab)}
          keys={["Assets", "Transactions"]}
        />

        <div className="max-w-full overflow-x-auto scrollbar-none">
          {activeTab === "Assets" && (
            <AssetsTable
              balances={balances}
              classNames={{ base: "p-0 shadow-none bg-transparent" }}
            />
          )}
          {activeTab === "Transactions" && (
            <TransactionsTable
              transactions={transactions}
              classNames={{ base: "p-0 shadow-none bg-transparent" }}
            />
          )}
        </div>
      </div>
    </div>
  );
};

// ============================================================================
// NotFound Component
// ============================================================================

const NotFound: React.FC = () => {
  const { username, profile, isLoading } = useUserExplorer();

  if (isLoading || profile) return null;

  return (
    <div className="w-full md:max-w-[76rem] p-4">
      <HeaderExplorer>
        <div className="flex flex-col gap-2 items-center">
          <h3 className="exposure-m-italic text-ink-secondary-700">User Not Found</h3>
          <p className="diatype-m-medium max-w-[42.5rem] text-center text-ink-tertiary-500">
            The user <span className="underline">{username}</span> could not be found.
          </p>
        </div>
      </HeaderExplorer>
    </div>
  );
};

// ============================================================================
// Export Compound Component
// ============================================================================

export const UserExplorer = Object.assign(Root, {
  Header,
  Content,
  NotFound,
});
