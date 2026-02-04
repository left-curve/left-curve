import { Cell, Pagination, Tab, Table, Tabs, twMerge } from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";
import type { ReferralMode } from "./ReferralStats";

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

type StatisticsData = {
  date: string;
  values: number[];
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

const mockStatisticsData: StatisticsData[] = [
  { date: "2024-01-02", values: [2500, 1500, 800] },
  { date: "2024-01-06", values: [1800, 1200, 600] },
  { date: "2024-01-10", values: [3200, 2000, 1000] },
  { date: "2024-01-14", values: [2800, 1800, 900] },
  { date: "2024-01-18", values: [4200, 2500, 1200] },
  { date: "2024-01-22", values: [3500, 2200, 1100] },
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

const BAR_COLORS = ["bg-[#C5C76E]", "bg-[#A8AA4A]", "bg-[#8B8D3D]"];

const StatisticsChart: React.FC = () => {
  const [metric, setMetric] = useState<"commission" | "volume">("commission");
  const [period, setPeriod] = useState<"7D" | "30D" | "90D">("7D");

  const maxValue = Math.max(...mockStatisticsData.flatMap((d) => d.values));
  const totalValue = mockStatisticsData.reduce(
    (acc, d) => acc + d.values.reduce((a, b) => a + b, 0),
    0,
  );

  return (
    <div className="p-4 lg:p-6 bg-surface-primary-rice">
      <div className="flex justify-between items-center mb-6">
        <div className="flex items-center gap-2">
          <select
            value={metric}
            onChange={(e) => setMetric(e.target.value as "commission" | "volume")}
            className="px-3 py-1.5 rounded-lg border border-outline-secondary-gray bg-surface-tertiary-rice text-ink-primary-900 diatype-s-medium cursor-pointer"
          >
            <option value="commission">Commission</option>
            <option value="volume">Volume</option>
          </select>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-ink-tertiary-500 diatype-s-medium">Period:</span>
          <select
            value={period}
            onChange={(e) => setPeriod(e.target.value as "7D" | "30D" | "90D")}
            className="px-3 py-1.5 rounded-lg border border-outline-secondary-gray bg-surface-tertiary-rice text-ink-primary-900 diatype-s-medium cursor-pointer"
          >
            <option value="7D">7D</option>
            <option value="30D">30D</option>
            <option value="90D">90D</option>
          </select>
        </div>
      </div>

      <div className="relative h-[200px] lg:h-[280px]">
        <div className="absolute top-0 right-0 flex flex-col items-end">
          <p className="text-ink-tertiary-500 diatype-xs-medium">Jul 17, 2025</p>
          <p className="text-ink-primary-900 diatype-m-bold">${totalValue.toLocaleString()}</p>
          <p className="text-primitives-warning-500 diatype-xs-medium">$1,791</p>
        </div>

        <div className="flex items-end justify-between h-full pt-12 pb-8 gap-2 lg:gap-4">
          {mockStatisticsData.map((data) => {
            const totalHeight = data.values.reduce((a, b) => a + b, 0);
            const heightPercent = (totalHeight / maxValue) * 100;

            return (
              <div key={data.date} className="flex-1 flex flex-col items-center gap-2">
                <div
                  className="w-full max-w-[60px] flex flex-col-reverse rounded-t-sm overflow-hidden"
                  style={{ height: `${heightPercent}%` }}
                >
                  {data.values.map((value, i) => {
                    const segmentPercent = (value / totalHeight) * 100;
                    return (
                      <div
                        key={`${data.date}-${i}`}
                        className={twMerge("w-full", BAR_COLORS[i % BAR_COLORS.length])}
                        style={{ height: `${segmentPercent}%` }}
                      />
                    );
                  })}
                </div>
                <p className="text-ink-tertiary-500 diatype-xs-medium whitespace-nowrap">
                  {data.date.slice(5)}
                </p>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};

type MyCommissionProps = {
  mode: ReferralMode;
};

export const MyCommission: React.FC<MyCommissionProps> = ({ mode }) => {
  const [affiliateTab, setAffiliateTab] = useState<CommissionTab>("my-commission");
  const [traderTab, setTraderTab] = useState<RebateTab>("my-rebates");

  const isAffiliate = mode === "affiliate";

  return (
    <div className="w-full flex flex-col rounded-xl border border-outline-secondary-gray overflow-hidden bg-surface-primary-gray">
      <div className="p-4 pb-0">
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
      </div>
      {isAffiliate && affiliateTab === "my-commission" && <CommissionTable />}
      {isAffiliate && affiliateTab === "my-referees" && <MyRefereesTable />}
      {isAffiliate && affiliateTab === "statistics" && <StatisticsChart />}
      {!isAffiliate && traderTab === "my-rebates" && <RebateTable />}
      {!isAffiliate && traderTab === "statistics" && <StatisticsChart />}
    </div>
  );
};
