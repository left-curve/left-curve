/** biome-ignore-all lint/suspicious/noArrayIndexKey: <explanation> */
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
  ReferenceLine,
} from "recharts";
import { FormattedNumber, Select, Spinner } from "@left-curve/applets-kit";
import { useVaultSnapshots } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { VaultPerformancePeriod, VaultPerformancePoint } from "@left-curve/store";

const PERIODS: VaultPerformancePeriod[] = ["7D", "30D", "90D"];

const formatDate = (date: string) => {
  const d = new Date(date);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
};

const formatFullDate = (date: string) => {
  const d = new Date(date);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
};

const formatPercent = (value: number) => `${value >= 0 ? "+" : ""}${value.toFixed(1)}%`;

function ChartTooltip({
  active,
  payload,
}: {
  active?: boolean;
  payload?: Array<{ payload: VaultPerformancePoint }>;
}) {
  if (!active || !payload?.[0]) return null;
  const data = payload[0].payload;
  return (
    <div className="bg-surface-primary-rice border border-outline-secondary-gray rounded-lg p-3 shadow-lg">
      <p className="diatype-xs-medium text-ink-tertiary-500 mb-1">{formatFullDate(data.date)}</p>
      <p className="diatype-sm-medium">
        <span className="text-utility-warning-600">{m["vaultLiquidity.price"]()}:</span>{" "}
        <FormattedNumber number={data.sharePrice.toString()} as="span" className="text-ink-secondary-700" />
      </p>
      <p className="diatype-sm-medium">
        <span
          className={data.dailyChange >= 0 ? "text-utility-success-600" : "text-utility-error-600"}
        >
          {m["vaultLiquidity.dailyChange"]()}:
        </span>{" "}
        <span
          className={data.dailyChange >= 0 ? "text-utility-success-600" : "text-utility-error-600"}
        >
          {formatPercent(data.dailyChange)}
        </span>
      </p>
    </div>
  );
}

const AXIS_TICK_STYLE = {
  fontSize: 11,
  fontFamily: "var(--font-diatype)",
  fill: "var(--color-ink-tertiary-500)",
};

function PriceTick({ x, y, payload }: { x?: number; y?: number; payload?: { value: number } }) {
  if (!payload) return null;
  return (
    <g transform={`translate(${x},${y})`}>
      <foreignObject x={0} y={-8} width={80} height={16}>
        <FormattedNumber
          number={payload.value.toString()}
          as="span"
          className="diatype-xs-medium text-ink-tertiary-500"
        />
      </foreignObject>
    </g>
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

      <div className="flex items-center justify-between">
        <p className="diatype-sm-regular text-ink-tertiary-500">
          {m["vaultLiquidity.shareTokenPrice"]()}{" "}
          <FormattedNumber number={latestPrice.toString()} as="span" className="diatype-sm-bold text-ink-secondary-700" />
        </p>
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

      <div className="flex gap-4 diatype-xs-medium text-ink-tertiary-500">
        <div className="flex items-center gap-1">
          <div className="w-8 h-[2px] bg-primitives-rice-light-500 rounded" />
          <span>{m["vaultLiquidity.price"]()}</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded-[2px] bg-[#AFB244] border-[0.5px] border-[#D2D184]" />
          <span>{m["vaultLiquidity.dailyGain"]()}</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded-[2px] bg-primitives-red-light-500 border-[0.5px] border-primitives-red-light-300" />
          <span>{m["vaultLiquidity.dailyLoss"]()}</span>
        </div>
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
              tick={AXIS_TICK_STYLE}
              stroke="var(--color-outline-secondary-gray)"
              tickLine={false}
            />
            <YAxis
              yAxisId="change"
              orientation="left"
              tickFormatter={(v: number) => `${v.toFixed(1)}%`}
              tick={AXIS_TICK_STYLE}
              stroke="var(--color-outline-secondary-gray)"
              tickLine={false}
              axisLine={false}
            />
            <YAxis
              yAxisId="price"
              orientation="right"
              tick={<PriceTick />}
              stroke="var(--color-outline-secondary-gray)"
              tickLine={false}
              axisLine={false}
              domain={["auto", "auto"]}
            />
            <Tooltip
              content={<ChartTooltip />}
              cursor={{ stroke: "var(--color-ink-tertiary-500)", strokeDasharray: "4 4" }}
            />
            <ReferenceLine yAxisId="change" y={0} stroke="var(--color-outline-secondary-gray)" />
            <Bar yAxisId="change" dataKey="dailyChange" radius={[2, 2, 0, 0]} barSize={30}>
              {data.map((entry, index) => (
                <Cell
                  key={`bar-${index}`}
                  fill={
                    entry.dailyChange >= 0
                      ? "#AFB244"
                      : "var(--color-primitives-red-light-500, #E54E6B)"
                  }
                  stroke={
                    entry.dailyChange >= 0
                      ? "#D2D184"
                      : "var(--color-primitives-red-light-300, #F9A9B2)"
                  }
                  strokeWidth={0.5}
                />
              ))}
            </Bar>
            <Line
              yAxisId="price"
              type="monotone"
              dataKey="sharePrice"
              stroke="#D4882C"
              strokeWidth={2}
              dot={{ r: 3, fill: "#D4882C", strokeWidth: 0 }}
              activeDot={{
                r: 5,
                fill: "#D4882C",
                strokeWidth: 2,
                stroke: "var(--color-surface-primary-rice)",
              }}
            />
          </ComposedChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
