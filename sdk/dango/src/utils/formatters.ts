import { Decimal } from "@left-curve/sdk/utils";

export type FormatNumberOptions = {
  language: string;
  currency?: string;
  style?: "decimal" | "currency";
  minimumTotalDigits?: number;
  maximumTotalDigits?: number;
  mask: keyof typeof formatNumberMask;
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

/**
 * Format a number with the given options.
 * @param amount The number to format.
 * @param options The formatting options.
 * @param options.currency The currency code. Ex: "USD".
 * @param options.language The language to use. Ex: "en-US".
 * @param options.maximumTotalDigits The maximum number of total digits.
 * @param options.minimumTotalDigits The minimum number of total digits.
 * @param options.mask The mask to use. Ex: 1 for "1,234.00", 2 for "1.234,00", 3 for "1234,00", 4 for "1 234,00".
 * @returns The formatted number.
 */
export function formatNumber(_amount_: number | string, options: FormatNumberOptions) {
  const { language, currency, maximumTotalDigits = 20, minimumTotalDigits = 0, mask = 1 } = options;

  const amount = Decimal(_amount_);

  const currencyOptions = currency
    ? ({
        currency,
        currencyDisplay: "narrowSymbol",
        style: "currency",
      } as const)
    : ({
        style: "decimal",
      } as const);

  const intlOptions: Intl.NumberFormatOptions = {
    ...currencyOptions,
    useGrouping: formatNumberMask[mask].useGrouping,
  };

  const absAmount = amount.abs();

  const integerPart = absAmount.round(0, 0);
  const integerDigits = integerPart.isZero() ? 1 : integerPart.toFixed(0).length;

  const threshold = Decimal(1).div(Decimal(10).pow(maximumTotalDigits - 1));

  if (absAmount.gt(0) && absAmount.lt(threshold)) {
    const thresholdFormatter = new Intl.NumberFormat(language, {
      maximumFractionDigits: maximumTotalDigits - 1,
    });
    return `< ${thresholdFormatter.format(threshold.toNumber())}`;
  }

  if (integerDigits > maximumTotalDigits) {
    intlOptions.notation = "compact";
    intlOptions.maximumFractionDigits = maximumTotalDigits;
  } else {
    intlOptions.maximumFractionDigits = maximumTotalDigits - integerDigits;
  }

  if (minimumTotalDigits > integerDigits) {
    intlOptions.minimumFractionDigits = minimumTotalDigits;
  } else {
    intlOptions.minimumFractionDigits = minimumTotalDigits - integerDigits;
  }

  return new Intl.NumberFormat(language, intlOptions)
    .formatToParts(amount.toNumber())
    .map((part) => {
      const partType =
        formatNumberMask[mask].format[
          part.type as keyof (typeof formatNumberMask)[keyof typeof formatNumberMask]["format"]
        ];
      return partType || part.value;
    })
    .join("");
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
 * @returns The parsed number.
 */
export function parseUnits(value: string, decimals: number): string {
  return Decimal(value).times(Decimal(10).pow(decimals)).toFixed(0);
}
