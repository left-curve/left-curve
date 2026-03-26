import {
  Cell,
  Pagination,
  Select,
  Skeleton,
  Tab,
  Table,
  Tabs,
  twMerge,
} from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import { useAccount, useRefereeStats, useWeeklyPoints, useUserVolume } from "@left-curve/store";
import type { RefereeStats } from "@left-curve/store";
import type React from "react";
import { Suspense, lazy, useMemo, useState } from "react";
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

/**
 * Format a number as USD currency
 */
const formatUSD = (value: number | string): string => {
  const num = typeof value === "string" ? Number(value) : value;
  if (Number.isNaN(num)) return "$0.00";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(num);
};

/**
 * Format a timestamp as a date string
 */
const formatDate = (timestamp: number): string => {
  return new Date(timestamp * 1000).toLocaleDateString("en-US", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
};

const NotConnectedMessage: React.FC = () => (
  <div className="p-8 bg-surface-primary-gray flex items-center justify-center">
    <p className="text-ink-tertiary-500 diatype-m-medium">Log in to view your data</p>
  </div>
);

const CommissionTable: React.FC = () => {
  const [currentPage, setCurrentPage] = useState(1);
  const { account, isConnected } = useAccount();
  const userIndex = account?.index;

  // Use weekly points to get commission history by week
  const { weeklyPoints, isLoading } = useWeeklyPoints({
    pointsUrl: "", // Will be set by the hook from config
    userIndex,
  });

  // Transform weekly points to commission rows
  const commissionData = useMemo<CommissionRow[]>(() => {
    if (!weeklyPoints) return [];

    return Object.entries(weeklyPoints).map(([week, points]) => {
      const weekNumber = Number.parseInt(week, 10);
      // Approximate date from week number (assuming epoch start)
      const weekDate = new Date();
      weekDate.setDate(weekDate.getDate() - 7 * (52 - weekNumber));

      return {
        myCommission: formatUSD(points.referral),
        referralVolume: "-", // TODO: Get from contract when available
        activeUsers: "-", // TODO: Get from contract when available
        date: weekDate.toLocaleDateString("en-US"),
      };
    });
  }, [weeklyPoints]);

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

  if (!isConnected) {
    return <NotConnectedMessage />;
  }

  if (isLoading) {
    return (
      <div className="p-4 bg-surface-primary-gray">
        <div className="space-y-3">
          {[...Array(5)].map((_, i) => (
            <Skeleton key={i} className="w-full h-12" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <Table
      data={commissionData}
      columns={columns}
      classNames={{ base: "shadow-none bg-surface-primary-gray" }}
      bottomContent={
        commissionData.length > 10 ? (
          <div className="p-4">
            <Pagination
              totalPages={Math.ceil(commissionData.length / 10)}
              currentPage={currentPage}
              onPageChange={setCurrentPage}
            />
          </div>
        ) : undefined
      }
    />
  );
};

const MyRefereesTable: React.FC = () => {
  const [currentPage, setCurrentPage] = useState(1);
  const { account, isConnected } = useAccount();
  const userIndex = account?.index;

  // Fetch referee stats from contract
  const { referees, isLoading } = useRefereeStats({
    referrerIndex: userIndex,
  });

  // Transform referee stats to table rows
  const refereeData = useMemo<RefereeRow[]>(() => {
    return referees.map((referee: RefereeStats) => ({
      userName: `#${referee.user_index}`, // Display user index (could be enhanced with username lookup)
      totalVolume: formatUSD(referee.volume),
      totalCommission: formatUSD(referee.commission),
      date: formatDate(referee.registered_at),
    }));
  }, [referees]);

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
      header: "Date Joined",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.date} />
      ),
    },
  ];

  if (!isConnected) {
    return <NotConnectedMessage />;
  }

  if (isLoading) {
    return (
      <div className="p-4 bg-surface-primary-gray">
        <div className="space-y-3">
          {[...Array(5)].map((_, i) => (
            <Skeleton key={i} className="w-full h-12" />
          ))}
        </div>
      </div>
    );
  }

  if (refereeData.length === 0) {
    return (
      <div className="p-8 bg-surface-primary-gray flex items-center justify-center">
        <p className="text-ink-tertiary-500 diatype-m-medium">No referees yet</p>
      </div>
    );
  }

  return (
    <Table
      data={refereeData}
      columns={columns}
      classNames={{ base: "shadow-none bg-surface-primary-gray" }}
      bottomContent={
        refereeData.length > 10 ? (
          <div className="p-4">
            <Pagination
              totalPages={Math.ceil(refereeData.length / 10)}
              currentPage={currentPage}
              onPageChange={setCurrentPage}
            />
          </div>
        ) : undefined
      }
    />
  );
};

const RebateTable: React.FC = () => {
  const [currentPage, setCurrentPage] = useState(1);
  const { account, isConnected } = useAccount();
  const userIndex = account?.index;

  // Get user's trading volume
  const { volume, isLoading } = useUserVolume({
    userIndex,
    days: 30,
  });

  // TODO: Get actual rebate data from contract when available
  // For now, show a placeholder message
  const rebateData = useMemo<RebateRow[]>(() => {
    // Placeholder - will be populated when contract provides rebate history
    if (volume && volume > 0) {
      return [
        {
          rebates: "$0.00", // TODO: Get from contract
          tradingVolume: formatUSD(volume),
          date: new Date().toLocaleDateString("en-US"),
        },
      ];
    }
    return [];
  }, [volume]);

  const columns: TableColumn<RebateRow> = [
    {
      header: "Rebates",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.rebates} />
      ),
    },
    {
      header: "Trading Volume",
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.tradingVolume}
        />
      ),
    },
    {
      header: "Date",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.date} />
      ),
    },
  ];

  if (!isConnected) {
    return <NotConnectedMessage />;
  }

  if (isLoading) {
    return (
      <div className="p-4 bg-surface-primary-gray">
        <div className="space-y-3">
          {[...Array(5)].map((_, i) => (
            <Skeleton key={i} className="w-full h-12" />
          ))}
        </div>
      </div>
    );
  }

  if (rebateData.length === 0) {
    return (
      <div className="p-8 bg-surface-primary-gray flex items-center justify-center">
        <p className="text-ink-tertiary-500 diatype-m-medium">No rebate history yet</p>
      </div>
    );
  }

  return (
    <Table
      data={rebateData}
      columns={columns}
      classNames={{ base: "shadow-none bg-surface-primary-gray" }}
      bottomContent={
        rebateData.length > 10 ? (
          <div className="p-4">
            <Pagination
              totalPages={Math.ceil(rebateData.length / 10)}
              currentPage={currentPage}
              onPageChange={setCurrentPage}
            />
          </div>
        ) : undefined
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
  const { isConnected } = useAccount();
  const [affiliateTab, setAffiliateTab] = useState<CommissionTab>("my-commission");
  const [traderTab, setTraderTab] = useState<RebateTab>("my-rebates");
  const [chartMetric, setChartMetric] = useState<ChartMetric>("commission");
  const [chartPeriod, setChartPeriod] = useState<ChartPeriod>("7D");

  if (!isConnected) return;

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
