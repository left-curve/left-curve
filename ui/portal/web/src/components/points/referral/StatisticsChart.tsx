import type React from "react";
import { useMemo } from "react";
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from "recharts";
import { useApp } from "@left-curve/applets-kit";
import { formatNumber } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useEpochPoints } from "@left-curve/store";

type ChartDataPoint = {
  date: string;
  value: number;
};

const BAR_COLOR = "#A8AA4A";

type CustomTooltipProps = {
  active?: boolean;
  payload?: Array<{ value: number }>;
  label?: string;
};

const formatValue = (value: number): string => {
  if (value >= 1000) {
    return `$${(value / 1000).toFixed(2)}k`;
  }
  return `$${value.toFixed(2)}`;
};

const formatDateLabel = (dateString: string): string => {
  const date = new Date(dateString);
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
};

const CustomTooltip: React.FC<CustomTooltipProps> = ({ active, payload, label }) => {
  if (!active || !payload?.length) return null;

  return (
    <div className="bg-surface-tertiary-gray rounded-lg shadow-lg p-3">
      <p className="text-ink-tertiary-500 diatype-xs-medium mb-2">
        {label && formatDateLabel(label)}
      </p>
      <div className="flex items-center gap-2">
        <span className="diatype-xs-medium" style={{ color: BAR_COLOR }}>
          {formatValue(payload[0].value)}
        </span>
      </div>
    </div>
  );
};

type StatisticsChartProps = {
  metric: "commission" | "volume";
  period: "7D" | "30D" | "90D";
};

const periodToDays: Record<string, number> = {
  "7D": 7,
  "30D": 30,
  "90D": 90,
};

export const StatisticsChart: React.FC<StatisticsChartProps> = ({ metric, period }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { account } = useAccount();
  const userIndex = account?.index;

  const pointsUrl = window.dango.urls.pointsUrl;
  const { epochPoints, isLoading } = useEpochPoints({
    pointsUrl,
    userIndex,
  });

  const chartData = useMemo<ChartDataPoint[]>(() => {
    if (!epochPoints) return [];

    const days = periodToDays[period];
    const cutoff = Date.now() / 1000 - days * 86400;

    return Object.entries(epochPoints)
      .map(([_epoch, epochStats]) => {
        const epochStartTs = Number.parseFloat(epochStats.started_at);
        const date = new Date(epochStartTs * 1000).toISOString().slice(0, 10);
        const value = metric === "commission"
          ? Number(epochStats.stats.points.referral)
          : Number(epochStats.stats.volume);

        return { date, value, ts: epochStartTs };
      })
      .filter((d) => d.ts >= cutoff)
      .sort((a, b) => a.ts - b.ts)
      .map(({ date, value }) => ({ date, value }));
  }, [epochPoints, metric, period]);

  if (isLoading) {
    return (
      <div className="p-4 bg-surface-primary-gray h-[15rem] lg:h-[28.125rem] flex items-center justify-center">
        <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.chart.loading"]()}</p>
      </div>
    );
  }

  if (chartData.length === 0) {
    return (
      <div className="p-4 bg-surface-primary-gray h-[15rem] lg:h-[28.125rem] flex items-center justify-center">
        <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.chart.noData"]()}</p>
      </div>
    );
  }

  return (
    <div className="p-4 bg-surface-primary-gray [&_*]:outline-none">
      <div className="h-[15rem] lg:h-[28.125rem]">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={chartData} barCategoryGap="15%">
            <CartesianGrid
              strokeDasharray="0"
              vertical={false}
              stroke="var(--color-outline-secondary-rice)"
              strokeWidth={1}
            />
            <XAxis
              dataKey="date"
              axisLine={false}
              tickLine={false}
              tick={{ fill: "var(--color-ink-secondary-700)", fontSize: 12 }}
              dy={10}
            />
            <YAxis
              axisLine={false}
              tickLine={false}
              tick={{ fill: "var(--color-ink-secondary-700)", fontSize: 12 }}
              tickFormatter={(value) => formatNumber(value, formatNumberOptions)}
              width={50}
            />
            <Tooltip content={<CustomTooltip />} cursor={{ fill: "transparent" }} />
            <Bar dataKey="value" fill={BAR_COLOR} radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

export default StatisticsChart;
