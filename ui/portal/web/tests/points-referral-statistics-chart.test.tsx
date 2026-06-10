import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { PropsWithChildren, ReactElement } from "react";

import { resetAppletsKitMocks } from "./mocks/applets-kit";

import { StatisticsChart } from "../src/components/points/referral/StatisticsChart";

type ChartDataPoint = {
  date: string;
  value: number;
};

const statisticsChartMocks = vi.hoisted(() => ({
  useAccount: vi.fn(),
  useEpochPoints: vi.fn(),
}));

vi.mock("recharts", async () => {
  const React = await import("react");

  return {
    Bar: ({ dataKey, fill }: { dataKey: string; fill: string }) => (
      <div data-fill={fill} data-key={dataKey} data-testid="chart-bar" />
    ),
    BarChart: ({
      children,
      data,
    }: PropsWithChildren<{
      data: ChartDataPoint[];
    }>) => (
      <div data-chart={JSON.stringify(data)} data-testid="bar-chart">
        {children}
      </div>
    ),
    CartesianGrid: () => <div data-testid="chart-grid" />,
    ResponsiveContainer: ({ children }: PropsWithChildren) => (
      <div data-testid="responsive-container">{children}</div>
    ),
    Tooltip: ({ content }: { content?: ReactElement }) => (
      <div data-testid="chart-tooltip">
        {content
          ? React.cloneElement(content, {
              active: true,
              label: "2026-06-07",
              payload: [{ value: 1200 }],
            })
          : null}
      </div>
    ),
    XAxis: ({ dataKey }: { dataKey: string }) => <div data-key={dataKey} data-testid="x-axis" />,
    YAxis: ({ tickFormatter }: { tickFormatter?: (value: number) => string }) => (
      <div data-testid="y-axis">{tickFormatter?.(1200)}</div>
    ),
  };
});

vi.mock("@left-curve/store", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/store")>();

  return {
    ...actual,
    useAccount: statisticsChartMocks.useAccount,
    useEpochPoints: statisticsChartMocks.useEpochPoints,
  };
});

function epochPoint({
  epoch,
  referral,
  startedAt,
  volume,
}: {
  epoch: number;
  referral: string;
  startedAt: string;
  volume: string;
}) {
  const startedAtSeconds = Date.parse(startedAt) / 1000;

  return [
    epoch,
    {
      ended_at: String(startedAtSeconds + 86_400),
      started_at: String(startedAtSeconds),
      stats: {
        points: {
          perps: "0",
          referral,
          vault: "0",
        },
        realized_pnl: "0",
        volume,
      },
    },
  ] as const;
}

function setEpochPoints({
  epochPoints,
  isLoading = false,
}: {
  epochPoints?: unknown[];
  isLoading?: boolean;
} = {}) {
  statisticsChartMocks.useEpochPoints.mockReturnValue({
    epochPoints,
    isLoading,
  });
}

function chartData() {
  const serializedChartData = screen.getByTestId("bar-chart").getAttribute("data-chart");
  expect(serializedChartData).toBeTruthy();

  return JSON.parse(serializedChartData as string) as ChartDataPoint[];
}

describe("StatisticsChart", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-08T12:00:00.000Z"));

    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          pointsUrl: "https://points.example",
        },
      },
    });

    statisticsChartMocks.useAccount.mockReturnValue({
      userIndex: 42,
    });
    setEpochPoints();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("requests epoch points for the connected account and renders the loading state", () => {
    setEpochPoints({
      isLoading: true,
    });

    render(<StatisticsChart metric="commission" period="7D" />);

    expect(statisticsChartMocks.useEpochPoints).toHaveBeenCalledWith({
      pointsUrl: "https://points.example",
      userIndex: 42,
    });
    expect(screen.getByText(m["referral.chart.loading"]())).toBeInTheDocument();
    expect(screen.queryByTestId("bar-chart")).not.toBeInTheDocument();
  });

  it("shows the empty state when there are no epochs in the selected period", () => {
    setEpochPoints({
      epochPoints: [
        epochPoint({
          epoch: 1,
          referral: "500",
          startedAt: "2026-05-20T00:00:00.000Z",
          volume: "5000",
        }),
      ],
    });

    render(<StatisticsChart metric="commission" period="7D" />);

    expect(screen.getByText(m["referral.chart.noData"]())).toBeInTheDocument();
    expect(screen.queryByTestId("bar-chart")).not.toBeInTheDocument();
  });

  it("filters 7D commission epochs, sorts by start time, and passes referral points to the chart", () => {
    setEpochPoints({
      epochPoints: [
        epochPoint({
          epoch: 3,
          referral: "70",
          startedAt: "2026-06-07T00:00:00.000Z",
          volume: "7000",
        }),
        epochPoint({
          epoch: 1,
          referral: "999",
          startedAt: "2026-05-20T00:00:00.000Z",
          volume: "999999",
        }),
        epochPoint({
          epoch: 2,
          referral: "30",
          startedAt: "2026-06-03T00:00:00.000Z",
          volume: "3000",
        }),
      ],
    });

    render(<StatisticsChart metric="commission" period="7D" />);

    expect(chartData()).toEqual([
      { date: "2026-06-03", value: 30 },
      { date: "2026-06-07", value: 70 },
    ]);
    expect(screen.getByTestId("chart-bar")).toHaveAttribute("data-key", "value");
    expect(screen.getByTestId("chart-bar")).toHaveAttribute("data-fill", "#A8AA4A");
    expect(screen.getByText("Jun 7, 2026")).toBeInTheDocument();
    expect(screen.getByText("$1.20k")).toBeInTheDocument();
  });

  it("keeps zero-value backend commission epochs inside the selected period", () => {
    setEpochPoints({
      epochPoints: [
        epochPoint({
          epoch: 4,
          referral: "0",
          startedAt: "2026-06-04T00:00:00.000Z",
          volume: "4000",
        }),
        epochPoint({
          epoch: 5,
          referral: "12",
          startedAt: "2026-06-06T00:00:00.000Z",
          volume: "6000",
        }),
      ],
    });

    render(<StatisticsChart metric="commission" period="7D" />);

    expect(chartData()).toEqual([
      { date: "2026-06-04", value: 0 },
      { date: "2026-06-06", value: 12 },
    ]);
    expect(screen.queryByText(m["referral.chart.noData"]())).not.toBeInTheDocument();
  });

  it("uses volume values for the 30D metric without reading referral points", () => {
    setEpochPoints({
      epochPoints: [
        epochPoint({
          epoch: 3,
          referral: "1",
          startedAt: "2026-06-07T00:00:00.000Z",
          volume: "7000",
        }),
        epochPoint({
          epoch: 1,
          referral: "999",
          startedAt: "2026-04-01T00:00:00.000Z",
          volume: "999999",
        }),
        epochPoint({
          epoch: 2,
          referral: "1",
          startedAt: "2026-05-20T00:00:00.000Z",
          volume: "120000",
        }),
      ],
    });

    render(<StatisticsChart metric="volume" period="30D" />);

    expect(chartData()).toEqual([
      { date: "2026-05-20", value: 120000 },
      { date: "2026-06-07", value: 7000 },
    ]);
  });

  it("keeps zero-value backend volume epochs inside the selected period", () => {
    setEpochPoints({
      epochPoints: [
        epochPoint({
          epoch: 6,
          referral: "100",
          startedAt: "2026-06-05T00:00:00.000Z",
          volume: "0",
        }),
        epochPoint({
          epoch: 7,
          referral: "200",
          startedAt: "2026-06-07T00:00:00.000Z",
          volume: "2500",
        }),
      ],
    });

    render(<StatisticsChart metric="volume" period="7D" />);

    expect(chartData()).toEqual([
      { date: "2026-06-05", value: 0 },
      { date: "2026-06-07", value: 2500 },
    ]);
    expect(screen.queryByText(m["referral.chart.noData"]())).not.toBeInTheDocument();
  });

  it("includes 90D epochs at the cutoff and excludes older backend timestamps", () => {
    setEpochPoints({
      epochPoints: [
        epochPoint({
          epoch: 1,
          referral: "999",
          startedAt: "2026-03-10T11:59:59.000Z",
          volume: "999999",
        }),
        epochPoint({
          epoch: 2,
          referral: "45",
          startedAt: "2026-03-10T12:00:00.000Z",
          volume: "4500",
        }),
        epochPoint({
          epoch: 3,
          referral: "80",
          startedAt: "2026-06-08T00:00:00.000Z",
          volume: "8000",
        }),
      ],
    });

    render(<StatisticsChart metric="commission" period="90D" />);

    expect(chartData()).toEqual([
      { date: "2026-03-10", value: 45 },
      { date: "2026-06-08", value: 80 },
    ]);
  });
});
