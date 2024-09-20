import type { Language } from "@leftcurve/types";

export type CurrencyFormatterOptions = {
  currency: string;
  language: Language;
  maxFractionDigits?: number;
  minFractionDigits?: number;
};

/**
 * Format a currency with the given options.
 * @param amount The amount to format.
 * @param options The formatting options.
 * @param options.currency The currency code. Ex: "USD".
 * @param options.language The language to use. Ex: "en-US".
 * @param options.maxFractionDigits The maximum number of fraction digits.
 * @param options.minFractionDigits The minimum number of fraction digits.
 * @returns The formatted currency.
 */
export function formatCurrency(amount: number | bigint, options: CurrencyFormatterOptions) {
  const { currency, language, minFractionDigits = 2, maxFractionDigits = 2 } = options;
  return new Intl.NumberFormat(language, {
    currency,
    style: "currency",
    notation: "compact",
    minimumFractionDigits: minFractionDigits,
    maximumFractionDigits: maxFractionDigits,
    currencyDisplay: "narrowSymbol",
  }).format(amount);
}

export type NumberFormatterOptions = {
  language: Language;
  maxFractionDigits?: number;
  minFractionDigits?: number;
};

/**
 * Format a number with the given options.
 * @param amount The number to format.
 * @param options The formatting options.
 * @param options.language The language to use. Ex: "en-US".
 * @param options.maxFractionDigits The maximum number of fraction digits.
 * @param options.minFractionDigits The minimum number of fraction digits.
 * @returns The formatted number.
 */
export function formatNumber(_amount_: number | bigint | string, options: NumberFormatterOptions) {
  const { language, maxFractionDigits = 2, minFractionDigits = 2 } = options;
  const amount = typeof _amount_ === "string" ? BigInt(_amount_) : _amount_;
  return new Intl.NumberFormat(language, {
    notation: "compact",
    minimumFractionDigits: minFractionDigits,
    maximumFractionDigits: maxFractionDigits,
  }).format(amount);
}

/**
 * Format an address.
 * @param address The address to format.
 * @param substring The number of characters to show at the end.
 * @returns The formatted address.
 */
export function formatAddress(address: string, substring = 4): string {
  return address.slice(0, 6).concat("...") + address.substring(address.length - substring);
}
