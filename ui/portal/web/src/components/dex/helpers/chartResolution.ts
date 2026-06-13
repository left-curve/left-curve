import { CandleInterval } from "@left-curve/types";

import type { CandleIntervals } from "@left-curve/types";
import type { ResolutionString } from "@left-curve/tradingview";

const SECOND_MS = 1_000;
const MINUTE_MS = 60 * SECOND_MS;
const DAY_MS = 24 * 60 * MINUTE_MS;

export const CHART_RESOLUTIONS: readonly ResolutionString[] = [
  "1S" as ResolutionString,
  "1" as ResolutionString,
  "5" as ResolutionString,
  "15" as ResolutionString,
  "60" as ResolutionString,
  "240" as ResolutionString,
  "1D" as ResolutionString,
  "1W" as ResolutionString,
];

export function convertResolutionToCandleInterval(resolution: ResolutionString): CandleIntervals {
  if (resolution.includes("S")) return CandleInterval.OneSecond;
  if (resolution.includes("W")) return CandleInterval.OneWeek;
  if (resolution.includes("D")) return CandleInterval.OneDay;

  const minutes = Number.parseInt(resolution, 10);
  if (Number.isNaN(minutes)) throw new Error(`Unsupported resolution: ${resolution}`);

  switch (minutes) {
    case 1:
      return CandleInterval.OneMinute;
    case 5:
      return CandleInterval.FiveMinutes;
    case 15:
      return CandleInterval.FifteenMinutes;
    case 60:
      return CandleInterval.OneHour;
    case 240:
      return CandleInterval.FourHours;
    default:
      throw new Error(`Unsupported resolution in minutes: ${minutes}`);
  }
}

export function getChartResolutionBarTime(
  fillTimeMs: number,
  resolution: string,
): number | undefined {
  if (!Number.isFinite(fillTimeMs)) return undefined;

  // Keep in sync with dango/indexer/clickhouse/src/entities/candle_interval.rs::interval_start.
  if (resolution.includes("W")) {
    const fillDate = new Date(fillTimeMs);
    const dayStartMs = Date.UTC(
      fillDate.getUTCFullYear(),
      fillDate.getUTCMonth(),
      fillDate.getUTCDate(),
    );

    return (dayStartMs - fillDate.getUTCDay() * DAY_MS) / SECOND_MS;
  }

  const intervalMs = getResolutionIntervalMs(resolution);
  if (!intervalMs) return undefined;

  return (Math.floor(fillTimeMs / intervalMs) * intervalMs) / SECOND_MS;
}

function getResolutionIntervalMs(resolution: string): number | undefined {
  if (resolution.includes("S")) {
    const seconds = Number.parseInt(resolution, 10) || 1;
    return seconds * SECOND_MS;
  }

  if (resolution.includes("D")) return DAY_MS;

  const minutes = Number.parseInt(resolution, 10);
  if (Number.isNaN(minutes)) return undefined;

  return minutes * MINUTE_MS;
}
