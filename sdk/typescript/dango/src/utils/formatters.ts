import { Decimal } from "@left-curve/sdk/utils";

export type DisplayPart = {
  type: "integer" | "decimal" | "group" | "fraction" | "subscript" | "suffix" | "literal";
  value: string;
};

export type FormatNumberOptions = {
  language: string;
  currency?: string;
  style?: "decimal" | "currency";
  mask: keyof typeof formatNumberMask;
  /** Override: use exactly this many fraction digits, bypassing tier logic. */
  fractionDigits?: number;
};

const formatNumberMask = {
  // 1,234.00
  1: {
    useGrouping: true,
    format: {
      group: ",",
      decimal: ".",
    },
  },
  // 1.234,00
  2: {
    useGrouping: true,
    format: {
      group: ".",
      decimal: ",",
    },
  },
  // 1234,00
  3: {
    useGrouping: false,
    format: {
      decimal: ",",
    },
  },
  // 1 234,00
  4: {
    useGrouping: true,
    format: {
      group: " ",
      decimal: ",",
    },
  },
};

const SUBSCRIPT_DIGITS = "₀₁₂₃₄₅₆₇₈₉";

function toUnicodeSubscript(n: string): string {
  return [...n].map((c) => SUBSCRIPT_DIGITS[+c] ?? c).join("");
}

function mapIntlParts(
  parts: Intl.NumberFormatPart[],
  mask: keyof typeof formatNumberMask,
): DisplayPart[] {
  const format = formatNumberMask[mask].format as Record<string, string>;

  return parts.map((part): DisplayPart => {
    switch (part.type) {
      case "integer":
        return { type: "integer", value: part.value };
      case "fraction":
        return { type: "fraction", value: part.value };
      case "decimal":
        return { type: "decimal", value: format.decimal ?? part.value };
      case "group":
        return { type: "group", value: format.group ?? part.value };
      case "compact":
        return { type: "suffix", value: part.value };
      default:
        return { type: "literal", value: part.value };
    }
  });
}

function intlFormatToParts(
  value: number,
  language: string,
  currency: string | undefined,
  mask: keyof typeof formatNumberMask,
  intlOverrides: Intl.NumberFormatOptions,
): DisplayPart[] {
  const currencyOpts: Intl.NumberFormatOptions = currency
    ? { currency, currencyDisplay: "narrowSymbol", style: "currency" }
    : { style: "decimal" };

  const opts: Intl.NumberFormatOptions = {
    ...currencyOpts,
    useGrouping: formatNumberMask[mask].useGrouping,
    ...intlOverrides,
  };

  return mapIntlParts(new Intl.NumberFormat(language, opts).formatToParts(value), mask);
}

/**
 * Format a number into structured display parts using tier-based rules.
 *
 * Tiers:
 * 1. num < 0.0001 → subscript notation: 0.0ₙXXXX (4 sig digits)
 * 2. 0.0001 ≤ num < 1 → 4 significant digits
 * 3. 1 ≤ num < 100 → up to 4 decimal places
 * 4. 100 ≤ num < 10,000 → up to 2 decimal places + grouping
 * 5. 10,000 ≤ num < 1,000,000 → integer + grouping
 * 6. ≥ 1,000,000 → compact (M/B/T) + 2 decimal places
 *
 * When `fractionDigits` is set, tiers are bypassed and exactly that many
 * fraction digits are shown.
 */
export function formatDisplayNumber(
  _amount_: number | string,
  options: FormatNumberOptions,
): DisplayPart[] {
  const { language = "en-US", currency, mask = 1, fractionDigits } = options;
  const amount = Decimal(_amount_);
  const absAmount = amount.abs();

  // Zero
  if (absAmount.eq(0)) {
    if (currency) {
      return intlFormatToParts(0, language, currency, mask, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      });
    }
    return [{ type: "integer", value: "0" }];
  }

  // fractionDigits override — bypass tier logic
  if (fractionDigits !== undefined) {
    return intlFormatToParts(amount.toNumber(), language, currency, mask, {
      minimumFractionDigits: fractionDigits,
      maximumFractionDigits: fractionDigits,
    });
  }

  const numValue = amount.toNumber();

  // Tier 1: < 0.0001 — subscript notation
  if (absAmount.lt("0.0001")) {
    return formatSubscriptParts(amount, absAmount, { language, currency, mask });
  }

  // Tier 2: 0.0001 ≤ num < 1 — 4 significant digits
  if (absAmount.lt(1)) {
    return intlFormatToParts(numValue, language, currency, mask, {
      maximumSignificantDigits: 4,
      useGrouping: false,
    });
  }

  // Tier 3: 1 ≤ num < 100 — up to 4 decimal places
  if (absAmount.lt(100)) {
    return intlFormatToParts(numValue, language, currency, mask, {
      maximumFractionDigits: 4,
      useGrouping: false,
    });
  }

  // Tier 4: 100 ≤ num < 10,000 — up to 2 decimal places
  if (absAmount.lt(10000)) {
    return intlFormatToParts(numValue, language, currency, mask, {
      maximumFractionDigits: 2,
    });
  }

  // Tier 5: 10,000 ≤ num < 1,000,000 — integer
  if (absAmount.lt(1000000)) {
    return intlFormatToParts(numValue, language, currency, mask, {
      maximumFractionDigits: 0,
    });
  }

  // Tier 6: ≥ 1,000,000 — compact
  return intlFormatToParts(numValue, language, currency, mask, {
    notation: "compact",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function formatSubscriptParts(
  amount: ReturnType<typeof Decimal>,
  absAmount: ReturnType<typeof Decimal>,
  options: { language: string; currency?: string; mask: keyof typeof formatNumberMask },
): DisplayPart[] {
  const parts: DisplayPart[] = [];
  const format = formatNumberMask[options.mask].format as Record<string, string>;
  const decimalChar = format.decimal ?? ".";
  const isNegative = amount.lt(0);

  if (isNegative) parts.push({ type: "literal", value: "-" });

  if (options.currency) {
    const symbol =
      new Intl.NumberFormat(options.language, {
        style: "currency",
        currency: options.currency,
        currencyDisplay: "narrowSymbol",
      })
        .formatToParts(0)
        .find((p) => p.type === "currency")?.value ?? "";
    if (symbol) parts.push({ type: "literal", value: symbol });
  }

  // Count leading zeros from the exact decimal string
  const str = absAmount.toFixed();
  const frac = str.split(".")[1] ?? "";
  const leadingZeros = frac.match(/^0*/)?.[0].length ?? 0;

  // Round to leadingZeros + 4 decimal places for 4 significant digits
  const rounded = absAmount.toFixed(leadingZeros + 4);
  const roundedFrac = rounded.split(".")[1] ?? "";
  const roundedLeading = roundedFrac.match(/^0*/)?.[0].length ?? 0;
  const sigDigits = roundedFrac.slice(roundedLeading, roundedLeading + 4).padEnd(4, "0");

  parts.push(
    { type: "integer", value: "0" },
    { type: "decimal", value: decimalChar },
    { type: "integer", value: "0" },
    { type: "subscript", value: String(roundedLeading) },
    { type: "fraction", value: sigDigits },
  );

  return parts;
}

/**
 * Convert display parts to a flat string.
 * Subscript parts are rendered as unicode subscript digits.
 */
export function formatDisplayString(parts: DisplayPart[]): string {
  return parts
    .map((p) => (p.type === "subscript" ? toUnicodeSubscript(p.value) : p.value))
    .join("");
}

/**
 * Format a number with the given options.
 * @param amount The number to format.
 * @param options The formatting options.
 * @returns The formatted number as a string.
 */
export function formatNumber(_amount_: number | string, options: FormatNumberOptions) {
  return formatDisplayString(formatDisplayNumber(_amount_, options));
}

/**
 * Derive the number of fraction digits to display for prices based on
 * the selected bucket size.
 *
 * - bucket size >= 1 → 0 (integer prices)
 * - bucket size = 0.1 → 1
 * - bucket size = 0.01 → 2
 * - etc.
 */
export function bucketSizeToFractionDigits(bucketSize: string): number {
  if (Decimal(bucketSize).gte(1)) return 0;
  const str = Decimal(bucketSize).toFixed();
  const dotIndex = str.indexOf(".");
  if (dotIndex === -1) return 0;
  return str.length - dotIndex - 1;
}

/**
 * Truncate (don't round) a decimal string to at most `maxFraction` fractional
 * digits so we never exceed the user's intended/available value.
 *
 * Defaults to 6 digits to match on-chain `Dec<i128, 6>`.
 */
export function truncateDec(value: string, maxFraction = 6): string {
  const trimmed = value.trim();
  if (!trimmed) return trimmed;
  const negative = trimmed.startsWith("-");
  const unsigned = negative ? trimmed.slice(1) : trimmed;
  const [intPart, fracPart = ""] = unsigned.split(".");
  const truncated =
    fracPart.length > maxFraction ? `${intPart}.${fracPart.slice(0, maxFraction)}` : unsigned;
  return negative ? `-${truncated}` : truncated;
}

/**
 *  Divides a number by a given exponent of base 10 (10exponent), and formats it into a string representation of the number.
 * @param value The number to format.
 * @param decimals The number of decimals to divide the number by.
 * @returns The formatted number.
 */
export function formatUnits(value: number | string, decimals: number): string {
  return Decimal(value.toString()).div(Decimal(10).pow(decimals)).toFixed();
}

/**
 * Parses a string representation of a number with a given number of decimals.
 * @param value The string representation of the number.
 * @param decimals The number of decimals to divide the number by.
 * @param dp Whether to use decimal places or not. If true, it will return the number with decimal places, otherwise it will return the number without decimal places.
 * @returns The parsed number.
 */
export function parseUnits(value: string, decimals: number, dp?: boolean): string {
  const result = Decimal(value).times(Decimal(10).pow(decimals));
  return dp ? result.toFixed() : result.toFixed(0);
}
