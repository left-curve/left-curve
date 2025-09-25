import { format } from "date-fns";
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
