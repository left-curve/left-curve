/** biome-ignore-all lint/suspicious/noArrayIndexKey: <explanation> */
import {
  Cell,
  Pagination,
  Select,
  Skeleton,
  Tab,
  Table,
  Tabs,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import { formatNumber } from "@left-curve/dango/utils";
import { formatDate } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  useAccount,
  useRefereeStats,
  useReferralData,
  usePublicClient,
  useAppConfig,
  queryReferralData,
} from "@left-curve/store";
import { useQueries } from "@tanstack/react-query";
import type { RefereeStatsWithUser } from "@left-curve/store";
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



const ROWS_PER_PAGE = 10;
const SECONDS_PER_DAY = 86_400;

function dayBoundary(baseTs: number, daysAgo: number): number {
  const ts = baseTs - daysAgo * SECONDS_PER_DAY;
  return ts - (ts % SECONDS_PER_DAY);
}

function diffReferralData(
  wider: {
    commissionEarnedFromReferees?: string;
    refereesVolume?: string;
    cumulativeDailyActiveReferees?: number;
  },
  narrower: {
    commissionEarnedFromReferees?: string;
    refereesVolume?: string;
    cumulativeDailyActiveReferees?: number;
  },
) {
  return {
    commission:
      Number(wider.commissionEarnedFromReferees ?? "0") -
      Number(narrower.commissionEarnedFromReferees ?? "0"),
    volume: Number(wider.refereesVolume ?? "0") - Number(narrower.refereesVolume ?? "0"),
    activeUsers: (wider.cumulativeDailyActiveReferees ?? 0) - (narrower.cumulativeDailyActiveReferees ?? 0),
  };
}

const NotConnectedMessage: React.FC = () => (
  <div className="p-8 bg-surface-primary-gray flex items-center justify-center">
    <p className="text-ink-tertiary-500 diatype-m-medium">
      {m["referral.commission.logInToView"]()}
    </p>
  </div>
);

const CommissionTable: React.FC = () => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const formatUSD = (value: number | string) =>
    formatNumber(value, { ...formatNumberOptions, currency: "USD" });
  const [currentPage, setCurrentPage] = useState(1);
  const { userIndex, isConnected } = useAccount();
  const client = usePublicClient();
  const { data: appConfig } = useAppConfig();

  const nowTs = useMemo(() => Math.floor(Date.now() / 1000), []);

  const totalDays = 30;
  const totalPages = Math.ceil(totalDays / ROWS_PER_PAGE);

  const offset = (currentPage - 1) * ROWS_PER_PAGE;
  const rowsOnPage = Math.min(ROWS_PER_PAGE, totalDays - offset);

  const boundaries = useMemo(() => {
    const b: number[] = [];
    for (let i = rowsOnPage; i >= 0; i--) {
      b.push(dayBoundary(nowTs, offset + i));
    }
    return b;
  }, [nowTs, offset, rowsOnPage]);

  const queries = useQueries({
    queries: boundaries.map((since) => ({
      queryKey: ["referralData", userIndex, since],
      queryFn: () => queryReferralData(client!, appConfig.addresses.perps, userIndex!, since),
      enabled: !!client && !!userIndex,
    })),
  });

  const isLoading = queries.some((q) => q.isLoading);

  const commissionData = useMemo<CommissionRow[]>(() => {
    if (queries.some((q) => !q.data)) return [];

    const rows: CommissionRow[] = [];

    for (let i = 0; i < rowsOnPage; i++) {
      const wider = queries[i].data!;
      const narrower = queries[i + 1].data!;
      const delta = diffReferralData(wider, narrower);

      const dayTs = boundaries[i + 1];
      const dateStr = formatDate(new Date(dayTs * 1000), settings.dateFormat);

      rows.push({
        myCommission: formatUSD(delta.commission),
        referralVolume: formatUSD(delta.volume),
        activeUsers: String(delta.activeUsers),
        date: dateStr,
      });
    }

    return rows.reverse();
  }, [queries, boundaries, rowsOnPage]);

  const columns: TableColumn<CommissionRow> = [
    {
      header: m["referral.commission.columns.myCommission"](),
      cell: ({ row }) => <Cell.Text text={row.original.myCommission} />,
    },
    {
      header: m["referral.commission.columns.referralVolume"](),
      cell: ({ row }) => <Cell.Text text={row.original.referralVolume} />,
    },
    {
      header: m["referral.commission.columns.activeUsers"](),
      cell: ({ row }) => <Cell.Text text={row.original.activeUsers} />,
    },
    {
      header: m["referral.commission.columns.date"](),
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
          {Array.from({ length: 3 }, (_, i) => (
            <Skeleton key={`commission-skeleton-${i}`} className="w-full h-12" />
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
        totalPages > 1 ? (
          <div className="p-4">
            <Pagination
              totalPages={totalPages}
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
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const formatUSD = (value: number | string) =>
    formatNumber(value, { ...formatNumberOptions, currency: "USD" });
  const [currentPage, setCurrentPage] = useState(1);
  const { userIndex, isConnected } = useAccount();

  const { referees, isLoading } = useRefereeStats({
    referrerIndex: userIndex,
  });

  const refereeData = useMemo<RefereeRow[]>(() => {
    return referees.map((referee: RefereeStatsWithUser) => ({
      userName: `#${referee.userIndex}`,
      totalVolume: formatUSD(referee.volume),
      totalCommission: formatUSD(referee.commissionEarned),
      date: formatDate(new Date(referee.registeredAt * 1000), settings.dateFormat),
    }));
  }, [referees]);

  const columns: TableColumn<RefereeRow> = [
    {
      header: m["referral.commission.columns.userName"](),
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.userName} />
      ),
    },
    {
      header: m["referral.commission.columns.totalVolume"](),
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.totalVolume}
        />
      ),
    },
    {
      header: m["referral.commission.columns.totalCommission"](),
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.totalCommission}
        />
      ),
    },
    {
      header: m["referral.commission.columns.dateJoined"](),
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
        <p className="text-ink-tertiary-500 diatype-m-medium">
          {m["referral.commission.noReferees"]()}
        </p>
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
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const formatUSD = (value: number | string) =>
    formatNumber(value, { ...formatNumberOptions, currency: "USD" });
  const [currentPage, setCurrentPage] = useState(1);
  const { userIndex, isConnected } = useAccount();

  const { referralData, isLoading } = useReferralData({ userIndex });

  const rebateData = useMemo<RebateRow[]>(() => {
    const volume = Number(referralData?.volume ?? "0");
    const rebates = referralData?.commissionSharedByReferrer ?? "0";

    if (volume > 0 || Number(rebates) > 0) {
      return [
        {
          rebates: formatUSD(rebates),
          tradingVolume: formatUSD(volume),
          date: formatDate(new Date(), settings.dateFormat),
        },
      ];
    }
    return [];
  }, [referralData]);

  const columns: TableColumn<RebateRow> = [
    {
      header: m["referral.rebate.columns.rebates"](),
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.rebates} />
      ),
    },
    {
      header: m["referral.rebate.columns.tradingVolume"](),
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.tradingVolume}
        />
      ),
    },
    {
      header: m["referral.rebate.columns.date"](),
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
        <p className="text-ink-tertiary-500 diatype-m-medium">
          {m["referral.commission.noRebates"]()}
        </p>
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
    <p className="text-ink-tertiary-500 diatype-m-medium">
      {m["referral.commission.loadingChart"]()}
    </p>
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
            <Tab title="my-commission">{m["referral.commission.myCommission"]()}</Tab>
            <Tab title="my-referees">{m["referral.commission.myReferees"]()}</Tab>
            <Tab title="statistics">{m["referral.commission.statistics"]()}</Tab>
          </Tabs>
        ) : (
          <Tabs
            layoutId="rebate-tabs"
            selectedTab={traderTab}
            onTabChange={(value) => setTraderTab(value as RebateTab)}
          >
            <Tab title="my-rebates">{m["referral.rebate.myRebates"]()}</Tab>
            <Tab title="statistics">{m["referral.rebate.statistics"]()}</Tab>
          </Tabs>
        )}
        {showStatisticsSelects && (
          <div className="flex items-center gap-2">
            <Select
              value={chartMetric}
              onChange={(value) => setChartMetric(value as ChartMetric)}
              classNames={{ trigger: "max-h-[38px]" }}
            >
              <Select.Item value="commission">{m["referral.metric.commission"]()}</Select.Item>
              <Select.Item value="volume">{m["referral.metric.volume"]()}</Select.Item>
            </Select>
            <Select
              value={chartPeriod}
              onChange={(value) => setChartPeriod(value as ChartPeriod)}
              classNames={{ trigger: "max-h-[38px]" }}
            >
              <Select.Item value="7D">{m["referral.period.sevenDays"]()}</Select.Item>
              <Select.Item value="30D">{m["referral.period.thirtyDays"]()}</Select.Item>
              <Select.Item value="90D">{m["referral.period.ninetyDays"]()}</Select.Item>
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
