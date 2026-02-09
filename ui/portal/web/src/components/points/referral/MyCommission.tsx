import { Cell, Pagination, Select, Tab, Table, Tabs, twMerge } from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";
import { Suspense, lazy, useState } from "react";
import type { ReferralMode } from "./ReferralStats";

type ChartMetric = "commission" | "volume";
type ChartPeriod = "7D" | "30D" | "90D";

const StatisticsChart = lazy(() => import("./StatisticsChart"));

type CommissionTab = "my-commission" | "my-referees" | "statistics";
type RebateTab = "my-rebates" | "statistics";

type CommissionRow = {
  myCommission: string;
  referralVolume: string;
  activeUsers: string;
  date: string;
};

type RefereeRow = {
  userName: string;
  totalVolume: string;
  totalCommission: string;
  date: string;
};

type RebateRow = {
  rebates: string;
  tradingVolume: string;
  date: string;
};

const mockCommissionData: CommissionRow[] = [
  { myCommission: "$75.42", referralVolume: "$20.16", activeUsers: "3", date: "2024-05-03" },
  { myCommission: "$65.00", referralVolume: "$80.58", activeUsers: "3", date: "2024-05-03" },
  { myCommission: "$60.00", referralVolume: "$65.07", activeUsers: "0", date: "2024-05-03" },
  { myCommission: "$0.15", referralVolume: "$1.14", activeUsers: "0", date: "2024-05-05" },
  { myCommission: "$0.75", referralVolume: "$0.34", activeUsers: "2", date: "2024-05-06" },
  { myCommission: "$0.18", referralVolume: "$0.34", activeUsers: "0", date: "2024-05-06" },
  { myCommission: "$0.19", referralVolume: "$0", activeUsers: "0", date: "2024-05-07" },
  { myCommission: "$9.63", referralVolume: "$0", activeUsers: "0", date: "2024-05-08" },
  { myCommission: "$50.00", referralVolume: "$65.00", activeUsers: "1", date: "2024-05-09" },
  { myCommission: "$60.17", referralVolume: "$73.46", activeUsers: "0", date: "2024-05-10" },
];

const mockRefereeData: RefereeRow[] = [
  { userName: "Bearier", totalVolume: "$3,445.76", totalCommission: "$85.00", date: "2024-05-03" },
  { userName: "Lincoln", totalVolume: "$1,676.00", totalCommission: "$0.00", date: "2024-05-03" },
  { userName: "Jaxon", totalVolume: "$2,345.00", totalCommission: "$1.00", date: "2024-05-03" },
  { userName: "Quillan", totalVolume: "$423.00", totalCommission: "$0.00", date: "2024-05-03" },
  { userName: "Zainab", totalVolume: "$187.00", totalCommission: "$0.00", date: "2024-05-04" },
  { userName: "Tamsin", totalVolume: "$3,876.00", totalCommission: "$4.00", date: "2024-05-04" },
  { userName: "Persephone", totalVolume: "$0.00", totalCommission: "$4.00", date: "2024-05-05" },
  { userName: "Vesper", totalVolume: "$125.00", totalCommission: "$3.00", date: "2024-05-06" },
  { userName: "Remi", totalVolume: "$90.00", totalCommission: "$0.00", date: "2024-05-06" },
  { userName: "Thalassa", totalVolume: "$1,071", totalCommission: "$0.00", date: "2024-05-07" },
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
  const [currentPage, setCurrentPage] = useState(1);
  const columns: TableColumn<CommissionRow> = [
    {
      header: "My Commission",
      cell: ({ row }) => <Cell.Text text={row.original.myCommission} />,
    },
    {
      header: "Referral Volume",
      cell: ({ row }) => <Cell.Text text={row.original.referralVolume} />,
    },
    {
      header: "Active Users",
      cell: ({ row }) => <Cell.Text text={row.original.activeUsers} />,
    },
    {
      header: "Date",
      cell: ({ row }) => <Cell.Text text={row.original.date} />,
    },
  ];

  return (
    <Table
      data={mockCommissionData}
      columns={columns}
      classNames={{ base: "shadow-none bg-surface-primary-gray" }}
      bottomContent={
        <div className="p-4">
          <Pagination totalPages={10} currentPage={currentPage} onPageChange={setCurrentPage} />
        </div>
      }
    />
  );
};

const MyRefereesTable: React.FC = () => {
  const [currentPage, setCurrentPage] = useState(1);
  const columns: TableColumn<RefereeRow> = [
    {
      header: "User Name",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.userName} />
      ),
    },
    {
      header: "Total Volume",
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.totalVolume}
        />
      ),
    },
    {
      header: "Total Commission",
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.totalCommission}
        />
      ),
    },
    {
      header: "Date Only",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.date} />
      ),
    },
  ];

  return (
    <Table
      data={mockRefereeData}
      columns={columns}
      classNames={{ base: "shadow-none bg-surface-primary-gray" }}
      bottomContent={
        <div className="p-4">
          <Pagination totalPages={10} currentPage={currentPage} onPageChange={setCurrentPage} />
        </div>
      }
    />
  );
};

const RebateTable: React.FC = () => {
  const [currentPage, setCurrentPage] = useState(1);
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
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.tradingVolume}
        />
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
      classNames={{ base: "shadow-none bg-surface-primary-gray" }}
      bottomContent={
        <div className="p-4">
          <Pagination totalPages={10} currentPage={currentPage} onPageChange={setCurrentPage} />
        </div>
      }
    />
  );
};

const ChartLoading: React.FC = () => (
  <div className="p-4 lg:p-6 bg-surface-primary-gray h-[300px] flex items-center justify-center">
    <p className="text-ink-tertiary-500 diatype-m-medium">Loading chart...</p>
  </div>
);

type MyCommissionProps = {
  mode: ReferralMode;
};

export const MyCommission: React.FC<MyCommissionProps> = ({ mode }) => {
  const [affiliateTab, setAffiliateTab] = useState<CommissionTab>("my-commission");
  const [traderTab, setTraderTab] = useState<RebateTab>("my-rebates");
  const [chartMetric, setChartMetric] = useState<ChartMetric>("commission");
  const [chartPeriod, setChartPeriod] = useState<ChartPeriod>("7D");

  const isAffiliate = mode === "affiliate";
  const showStatisticsSelects =
    (isAffiliate && affiliateTab === "statistics") || (!isAffiliate && traderTab === "statistics");

  return (
    <div className="w-full flex flex-col rounded-xl border border-outline-secondary-gray overflow-hidden bg-surface-primary-gray">
      <div
        className={twMerge(
          "p-4 flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4",
          !showStatisticsSelects && "lg:pb-0",
        )}
      >
        {isAffiliate ? (
          <Tabs
            layoutId="commission-tabs"
            selectedTab={affiliateTab}
            onTabChange={(value) => setAffiliateTab(value as CommissionTab)}
          >
            <Tab title="my-commission">My Commission</Tab>
            <Tab title="my-referees">My Referees</Tab>
            <Tab title="statistics">Statistics</Tab>
          </Tabs>
        ) : (
          <Tabs
            layoutId="rebate-tabs"
            selectedTab={traderTab}
            onTabChange={(value) => setTraderTab(value as RebateTab)}
          >
            <Tab title="my-rebates">My Rebates</Tab>
            <Tab title="statistics">Statistics</Tab>
          </Tabs>
        )}
        {showStatisticsSelects && (
          <div className="flex items-center gap-2">
            <Select
              value={chartMetric}
              onChange={(value) => setChartMetric(value as ChartMetric)}
              classNames={{ trigger: "max-h-[38px]" }}
            >
              <Select.Item value="commission">Commission</Select.Item>
              <Select.Item value="volume">Volume</Select.Item>
            </Select>
            <Select
              value={chartPeriod}
              onChange={(value) => setChartPeriod(value as ChartPeriod)}
              classNames={{ trigger: "max-h-[38px]" }}
            >
              <Select.Item value="7D">Period: 7D</Select.Item>
              <Select.Item value="30D">Period: 30D</Select.Item>
              <Select.Item value="90D">Period: 90D</Select.Item>
            </Select>
          </div>
        )}
      </div>
      {isAffiliate && affiliateTab === "my-commission" && <CommissionTable />}
      {isAffiliate && affiliateTab === "my-referees" && <MyRefereesTable />}
      {isAffiliate && affiliateTab === "statistics" && (
        <Suspense fallback={<ChartLoading />}>
          <StatisticsChart metric={chartMetric} period={chartPeriod} />
        </Suspense>
      )}
      {!isAffiliate && traderTab === "my-rebates" && <RebateTable />}
      {!isAffiliate && traderTab === "statistics" && (
        <Suspense fallback={<ChartLoading />}>
          <StatisticsChart metric={chartMetric} period={chartPeriod} />
        </Suspense>
      )}
    </div>
  );
};
