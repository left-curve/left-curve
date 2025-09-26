import {
  differenceInDays,
  differenceInHours,
  differenceInMinutes,
  format,
  isToday,
} from "date-fns";
import { formatInTimeZone } from "date-fns-tz";

import type { DateArg, FormatOptions } from "date-fns";
import type { Prettify } from "@left-curve/dango/types";

/**
 *
 * Formats a date according to the given mask and options.
 * Supports local and UTC time zones.
 * @param date - The date to format (Date, string, or number)
 * @param mask - The format mask (e.g., "yyyy-MM-dd HH:mm:ss")
 * @param options - Optional formatting options including timeZone
 * @returns Formatted date string
 */
export function formatDate(
  date: DateArg<Date> & {},
  mask: string,
  options: Prettify<FormatOptions & { timeZone?: string }> = {},
) {
  const { timeZone = "local", ...formatOptions } = options;

  if (timeZone === "utc") return formatInTimeZone(date, "UTC", mask);

  return format(date, mask, formatOptions);
}

export const formatActivityTimestamp = (timestamp: Date, mask: string): string => {
  const now = new Date();
  if (isToday(timestamp)) {
    const minutesDifference = differenceInMinutes(now, timestamp);
    if (minutesDifference < 1) {
      return "1m";
    }

    if (minutesDifference < 60) {
      return `${minutesDifference}m`;
    }

    const hoursDifference = differenceInHours(now, timestamp);
    if (hoursDifference < 24) {
      return `${hoursDifference}h`;
    }
  }

  const daysDifference = differenceInDays(now, timestamp);
  if (daysDifference === 1) {
    return "1d";
  }

  return formatDate(timestamp, mask);
};
