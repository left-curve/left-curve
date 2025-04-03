export type CurrencyFormatterOptions = {
  currency: string;
  language: string;
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

export type FormatNumberOptions = {
  language: string;
  currency?: string;
  style?: "decimal" | "percent" | "currency";
  notation?: "standard" | "scientific" | "engineering" | "compact";
  maxFractionDigits?: number;
  minFractionDigits?: number;
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
 * @param options.maxFractionDigits The maximum number of fraction digits.
 * @param options.minFractionDigits The minimum number of fraction digits.
 * @returns The formatted number.
 */
export function formatNumber(_amount_: number | bigint | string, options: FormatNumberOptions) {
  const {
    language,
    currency,
    maxFractionDigits = 2,
    minFractionDigits = 2,
    notation = "standard",
    mask = 1,
  } = options;
  const amount = typeof _amount_ === "string" ? Number(_amount_) : _amount_;

  const currencyOptions = currency
    ? ({
        currency,
        currencyDisplay: "narrowSymbol",
        notation: "compact",
        style: "currency",
      } as const)
    : {};

  return new Intl.NumberFormat(language, {
    notation,
    // @ts-ignore: For some reason roundingMode is not in the type definition but it is supported.
    roundingMode: "floor",
    minimumFractionDigits: minFractionDigits,
    maximumFractionDigits: maxFractionDigits,
    useGrouping: formatNumberMask[mask].useGrouping,
    ...currencyOptions,
  })
    .formatToParts(amount)
    .map((part) => {
      const partType =
        formatNumberMask[mask].format[part.type as keyof (typeof formatNumberMask)[3]["format"]];
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
export function formatUnits(value: bigint | number | string, decimals: number): string {
  let display = typeof value === "string" ? value : value.toString();

  const negative = display.startsWith("-");
  if (negative) display = display.slice(1);

  display = display.padStart(decimals, "0");

  let [integer, fraction] = [
    display.slice(0, display.length - decimals),
    display.slice(display.length - decimals),
  ];
  fraction = fraction.replace(/(0+)$/, "");
  return `${negative ? "-" : ""}${integer || "0"}${fraction ? `.${fraction}` : ""}`;
}

/**
 * Parses a string representation of a number with a given number of decimals.
 * @param value The string representation of the number.
 * @param decimals The number of decimals to divide the number by.
 * @returns The parsed number.
 */
export function parseUnits(value: string, decimals: number) {
  if (!/^(-?)([0-9]*)\.?([0-9]*)$/.test(value)) {
    throw new Error(`Number \`${value}\` is not a valid decimal number.`);
  }

  let [integer, fraction = "0"] = value.split(".");

  const negative = integer.startsWith("-");
  if (negative) integer = integer.slice(1);

  // trim trailing zeros.
  fraction = fraction.replace(/(0+)$/, "");

  // round off if the fraction is larger than the number of decimals.
  if (decimals === 0) {
    if (Math.round(Number(`.${fraction}`)) === 1) integer = `${BigInt(integer) + BigInt(1)}`;
    fraction = "";
  } else if (fraction.length > decimals) {
    const [left, unit, right] = [
      fraction.slice(0, decimals - 1),
      fraction.slice(decimals - 1, decimals),
      fraction.slice(decimals),
    ];

    const rounded = Math.round(Number(`${unit}.${right}`));
    if (rounded > 9) fraction = `${BigInt(left) + BigInt(1)}0`.padStart(left.length + 1, "0");
    else fraction = `${left}${rounded}`;

    if (fraction.length > decimals) {
      fraction = fraction.slice(1);
      integer = `${BigInt(integer) + BigInt(1)}`;
    }

    fraction = fraction.slice(0, decimals);
  } else {
    fraction = fraction.padEnd(decimals, "0");
  }

  return BigInt(`${negative ? "-" : ""}${integer}${fraction}`);
}
