/**
 * Formats a date to the local time zone of the user's browser.
 *
 * @param {string | number | Date} date - The date to format, can be a string, number, or Date object.
 * @param {string} [locale="en-US"] - The locale to use for formatting. Defaults to "en-US".
 * @returns {Date} - A new Date object representing the date in the user's local time zone.
 */
export function formatToTimeZone(date: string | number | Date, locale = "en-US"): Date {
  const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;

  return new Date(new Date(date).toLocaleString(locale, { timeZone }));
}
