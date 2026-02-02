import { Cell, Tab, Table, Tabs } from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";
import type { ReferralMode } from "./ReferralStats";

type CommissionTab = "my-affiliates" | "my-referees" | "statistics";
type RebateTab = "my-rebates" | "statistics";

type CommissionRow = {
  myCommission: string;
  referralVolume: string;
  activeUsers: string;
  date: string;
};

type RebateRow = {
  rebates: string;
  tradingVolume: string;
  date: string;
};

const mockCommissionData: CommissionRow[] = [
  { myCommission: "$75.42", referralVolume: "$20.16", activeUsers: "3", date: "2024-05-01" },
  { myCommission: "$60.00", referralVolume: "$80.00", activeUsers: "3", date: "2024-02-01" },
  { myCommission: "$42.27", referralVolume: "$50.00", activeUsers: "0", date: "2024-02-07" },
  { myCommission: "$0.24", referralVolume: "$75.56", activeUsers: "0", date: "2024-06-09" },
  { myCommission: "$9.15", referralVolume: "$0.34", activeUsers: "2", date: "2024-04-03" },
  { myCommission: "$0.76", referralVolume: "$91.01", activeUsers: "0", date: "2024-01-03" },
  { myCommission: "$0.19", referralVolume: "$0", activeUsers: "0", date: "2024-06-02" },
  { myCommission: "$9.63", referralVolume: "$75.96", activeUsers: "0", date: "2024-08-07" },
  { myCommission: "$50.00", referralVolume: "$65.00", activeUsers: "1", date: "2024-06-08" },
  { myCommission: "$0.17", referralVolume: "$13.46", activeUsers: "...", date: "2024-01-01" },
];

const mockRebateData: RebateRow[] = [
  { rebates: "$20.10", tradingVolume: "$75.40", date: "2024-05-01" },
  { rebates: "$0.00", tradingVolume: "$80.00", date: "2024-02-01" },
  { rebates: "$42.27", tradingVolume: "$50.00", date: "2024-02-07" },
  { rebates: "$0.70", tradingVolume: "$75.56", date: "2024-06-09" },
  { rebates: "$0.10", tradingVolume: "$0.34", date: "2024-04-03" },
  { rebates: "$0.76", tradingVolume: "$91.01", date: "2024-01-03" },
  { rebates: "$0.19", tradingVolume: "$0", date: "2024-06-02" },
  { rebates: "$9.63", tradingVolume: "$75.96", date: "2024-08-07" },
  { rebates: "$50.00", tradingVolume: "$64.00", date: "2024-07-08" },
  { rebates: "$73.45", tradingVolume: "...", date: "2024-01-01" },
];

const CommissionTable: React.FC = () => {
  const columns: TableColumn<CommissionRow> = [
    {
      header: "My Commission ▼",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.myCommission} />
      ),
    },
    {
      header: "Referral Volume ▼",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.referralVolume} />
      ),
    },
    {
      header: "Active Users",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.activeUsers} />
      ),
    },
    {
      header: "Date ▼",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.date} />
      ),
    },
  ];

  return (
    <Table
      data={mockCommissionData}
      columns={columns}
      style="simple"
      classNames={{
        base: "bg-surface-primary-rice",
        header: "bg-surface-tertiary-rice text-ink-tertiary-500 diatype-s-medium border-b border-outline-secondary-gray",
        cell: "px-4 py-3",
        row: "border-b border-outline-secondary-gray last:border-b-0",
      }}
    />
  );
};

const RebateTable: React.FC = () => {
  const columns: TableColumn<RebateRow> = [
    {
      header: "Rebates ▼",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.rebates} />
      ),
    },
    {
      header: "Trading Volume ▼",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.tradingVolume} />
      ),
    },
    {
      header: "Date ▼",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.date} />
      ),
    },
  ];

  return (
    <Table
      data={mockRebateData}
      columns={columns}
      style="simple"
      classNames={{
        base: "bg-surface-primary-rice",
        header: "bg-surface-tertiary-rice text-ink-tertiary-500 diatype-s-medium border-b border-outline-secondary-gray",
        cell: "px-4 py-3",
        row: "border-b border-outline-secondary-gray last:border-b-0",
      }}
    />
  );
};

const StatisticsPlaceholder: React.FC = () => (
  <div className="p-8 flex items-center justify-center text-ink-tertiary-500 diatype-m-medium">
    Statistics will be available soon
  </div>
);

const MyRefereesPlaceholder: React.FC = () => (
  <div className="p-8 flex items-center justify-center text-ink-tertiary-500 diatype-m-medium">
    My Referees data will be available soon
  </div>
);

type MyCommissionProps = {
  mode: ReferralMode;
};

export const MyCommission: React.FC<MyCommissionProps> = ({ mode }) => {
  const [affiliateTab, setAffiliateTab] = useState<CommissionTab>("my-affiliates");
  const [traderTab, setTraderTab] = useState<RebateTab>("my-rebates");

  if (mode === "affiliate") {
    return (
      <div className="w-full flex flex-col rounded-xl border border-outline-secondary-gray overflow-hidden bg-surface-primary-gray">
        <div className="p-4 bg-surface-tertiary-rice">
          <Tabs
            layoutId="commission-tabs"
            selectedTab={affiliateTab}
            onTabChange={(value) => setAffiliateTab(value as CommissionTab)}
          >
            <Tab title="my-affiliates">My Affiliates</Tab>
            <Tab title="my-referees">My Referees</Tab>
            <Tab title="statistics">Statistics</Tab>
          </Tabs>
        </div>
        {affiliateTab === "my-affiliates" && <CommissionTable />}
        {affiliateTab === "my-referees" && <MyRefereesPlaceholder />}
        {affiliateTab === "statistics" && <StatisticsPlaceholder />}
        <div className="p-4 flex justify-center gap-2 text-ink-tertiary-500 diatype-s-medium">
          <span>1</span>
          <span>2</span>
          <span>3</span>
          <span>...</span>
          <span>8</span>
          <span>9</span>
          <span>10</span>
        </div>
      </div>
    );
  }

  return (
    <div className="w-full flex flex-col rounded-xl border border-outline-secondary-gray overflow-hidden bg-surface-primary-gray">
      <div className="p-4 bg-surface-tertiary-rice">
        <Tabs
          layoutId="rebate-tabs"
          selectedTab={traderTab}
          onTabChange={(value) => setTraderTab(value as RebateTab)}
        >
          <Tab title="my-rebates">My Rebates</Tab>
          <Tab title="statistics">Statistics</Tab>
        </Tabs>
      </div>
      {traderTab === "my-rebates" && <RebateTable />}
      {traderTab === "statistics" && <StatisticsPlaceholder />}
      <div className="p-4 flex justify-center gap-2 text-ink-tertiary-500 diatype-s-medium">
        <span>1</span>
        <span>2</span>
        <span>3</span>
        <span>...</span>
        <span>8</span>
        <span>9</span>
        <span>10</span>
      </div>
    </div>
  );
};
