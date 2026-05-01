import { useState } from "react";
import {
  ComposedChart,
  Bar,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
  CartesianGrid,
} from "recharts";
import { Select, Spinner } from "@left-curve/applets-kit";
import { useVaultSnapshots } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { VaultPerformancePeriod, VaultPerformancePoint } from "@left-curve/store";

const PERIODS: VaultPerformancePeriod[] = ["7D", "30D", "90D"];

const formatDate = (date: string) => {
  const d = new Date(date);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
};

const formatPrice = (value: number) =>
  `$${value.toLocaleString(undefined, { minimumFractionDigits: 3, maximumFractionDigits: 3 })}`;

const formatPercent = (value: number) => `${value >= 0 ? "+" : ""}${value.toFixed(1)}%`;

function ChartTooltip({ active, payload }: { active?: boolean; payload?: Array<{ payload: VaultPerformancePoint }> }) {
  if (!active || !payload?.[0]) return null;
  const data = payload[0].payload;
  return (
    <div className="bg-surface-primary-rice border border-outline-secondary-gray rounded-lg p-3 shadow-lg">
      <p className="diatype-xs-medium text-ink-tertiary-500 mb-1">{formatDate(data.date)}</p>
      <p className="diatype-sm-medium text-ink-secondary-700">
        {m["vaultLiquidity.price"]()}: {formatPrice(data.sharePrice)}
      </p>
      <p
        className={`diatype-sm-medium ${data.dailyChange >= 0 ? "text-utility-success-600" : "text-utility-error-600"}`}
      >
        {m["vaultLiquidity.dailyChange"]()}: {formatPercent(data.dailyChange)}
      </p>
    </div>
  );
}

export function VaultPerformanceChart() {
  const [period, setPeriod] = useState<VaultPerformancePeriod>("30D");
  const { data, isLoading, error } = useVaultSnapshots({ period });

  const latestPrice = data?.length ? data[data.length - 1].sharePrice : 0;

  return (
    <div className="flex flex-col gap-3 p-4 rounded-xl bg-surface-secondary-rice shadow-account-card">
      <p className="exposure-sm-italic text-ink-tertiary-500">
        {m["vaultLiquidity.performance"]()}
      </p>

      <div className="flex items-start justify-between">
        <div className="flex flex-col gap-1">
          <p className="diatype-sm-medium text-ink-secondary-700">
            {m["vaultLiquidity.shareTokenPrice"]()}{" "}
            <span className="diatype-sm-bold">{formatPrice(latestPrice)}</span>
          </p>
          <div className="flex gap-4 diatype-xs-medium text-ink-tertiary-500">
        <div className="flex items-center gap-1">
          <div className="w-3 h-[2px] bg-utility-warning-600 rounded" />
          <span>{m["vaultLiquidity.price"]()}</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 bg-utility-success-500 rounded-sm" />
          <span>{m["vaultLiquidity.dailyGain"]()}</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 bg-utility-error-300 rounded-sm" />
          <span>{m["vaultLiquidity.dailyLoss"]()}</span>
        </div>
          </div>
        </div>
        <Select
          value={period}
          onChange={(v) => setPeriod(v as VaultPerformancePeriod)}
          classNames={{
            listboxWrapper: "right-0",
            trigger: "py-2 px-4 max-w-[9.375rem]",
          }}
        >
          {PERIODS.map((p) => (
            <Select.Item key={p} value={p}>
              <span data-hide-in-dropdown className="diatype-m-regular text-ink-secondary-700">
                {m["vaultLiquidity.period"]()}:{" "}
              </span>
              <span className="diatype-m-bold text-ink-primary-900">{p}</span>
            </Select.Item>
          ))}
        </Select>
      </div>

      {error ? (
        <div className="flex items-center justify-center h-[200px] text-utility-error-600 diatype-sm-regular">
          {error.message}
        </div>
      ) : isLoading || !data ? (
        <div className="flex items-center justify-center h-[200px]">
          <Spinner color="pink" size="md" />
        </div>
      ) : data.length === 0 ? (
        <div className="flex items-center justify-center h-[200px] text-ink-tertiary-500 diatype-sm-regular">
          No data available
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={200}>
          <ComposedChart data={data} margin={{ top: 5, right: 0, left: 0, bottom: 0 }}>
            <CartesianGrid
              strokeDasharray="3 3"
              vertical={false}
              stroke="var(--color-outline-secondary-gray)"
            />
            <XAxis
              dataKey="date"
              tickFormatter={formatDate}
              tick={{ fontSize: 11 }}
              stroke="var(--color-outline-secondary-gray)"
              tickLine={false}
            />
            <YAxis
              yAxisId="change"
              orientation="left"
              tickFormatter={(v: number) => `${v.toFixed(1)}%`}
              tick={{ fontSize: 11 }}
              stroke="var(--color-outline-secondary-gray)"
              tickLine={false}
              axisLine={false}
            />
            <YAxis
              yAxisId="price"
              orientation="right"
              tickFormatter={(v: number) => `$${v.toFixed(3)}`}
              tick={{ fontSize: 11 }}
              stroke="var(--color-outline-secondary-gray)"
              tickLine={false}
              axisLine={false}
              domain={["auto", "auto"]}
            />
            <Tooltip content={<ChartTooltip />} />
            <Bar yAxisId="change" dataKey="dailyChange" radius={[2, 2, 0, 0]} maxBarSize={12}>
              {data.map((entry, index) => (
                <Cell
                  key={`bar-${index}`}
                  fill={
                    entry.dailyChange >= 0
                      ? "var(--color-utility-success-500, #22c55e)"
                      : "var(--color-utility-error-300, #fca5a5)"
                  }
                />
              ))}
            </Bar>
            <Line
              yAxisId="price"
              type="monotone"
              dataKey="sharePrice"
              stroke="var(--color-utility-warning-600, #d97706)"
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4 }}
            />
          </ComposedChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
