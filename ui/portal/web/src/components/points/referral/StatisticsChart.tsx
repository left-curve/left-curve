import type React from "react";
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from "recharts";

type StatisticsChartData = {
  date: string;
  tier1: number;
  tier2: number;
  tier3: number;
};

const mockChartData: StatisticsChartData[] = [
  { date: "2026-01-20", tier1: 250, tier2: 280, tier3: 320 },
  { date: "2026-01-21", tier1: 300, tier2: 300, tier3: 320 },
  { date: "2026-01-22", tier1: 260, tier2: 240, tier3: 280 },
  { date: "2026-01-23", tier1: 300, tier2: 280, tier3: 300 },
  { date: "2026-01-24", tier1: 150, tier2: 200, tier3: 350 },
  { date: "2026-01-25", tier1: 280, tier2: 300, tier3: 340 },
  { date: "2026-01-26", tier1: 280, tier2: 260, tier3: 320 },
];

const TIER_LABELS = ["Tamsin", "Oswald", "Scranton"];

const BAR_COLORS = ["#C5C76E", "#A8AA4A", "#8B8D3D"];

type CustomTooltipProps = {
  active?: boolean;
  payload?: Array<{ value: number; dataKey: string }>;
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
  if (!active || !payload) return null;

  return (
    <div className="bg-surface-tertiary-gray rounded-lg shadow-lg p-3">
      <p className="text-ink-tertiary-500 diatype-xs-medium mb-2">
        {label && formatDateLabel(label)}
      </p>
      <div className="flex flex-col gap-1">
        {payload.map((item, index) => (
          <div key={item.dataKey} className="flex items-center gap-2">
            <span className="diatype-xs-medium" style={{ color: BAR_COLORS[2 - index] }}>
              {TIER_LABELS[2 - index]}
            </span>
            <span className="text-ink-primary-900 diatype-xs-medium">
              {formatValue(item.value)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
};

type StatisticsChartProps = {
  metric: "commission" | "volume";
  period: "7D" | "30D" | "90D";
};

export const StatisticsChart: React.FC<StatisticsChartProps> = ({ metric, period }) => {
  return (
    <div className="p-4 bg-surface-primary-gray [&_*]:outline-none">
      <div className="h-[15rem] lg:h-[28.125rem]">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={mockChartData} barCategoryGap="15%">
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
              tickFormatter={(value) => value.toLocaleString()}
              width={50}
            />
            <Tooltip content={<CustomTooltip />} cursor={{ fill: "transparent" }} />
            <Bar dataKey="tier3" stackId="stack" fill={BAR_COLORS[2]} radius={[0, 0, 0, 0]} />
            <Bar dataKey="tier2" stackId="stack" fill={BAR_COLORS[1]} radius={[0, 0, 0, 0]} />
            <Bar dataKey="tier1" stackId="stack" fill={BAR_COLORS[0]} radius={[10, 10, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

export default StatisticsChart;
