import { useApp, twMerge } from "@left-curve/foundation";

import { Decimal, formatNumber } from "@left-curve/dango/utils";

import type React from "react";
import type { FormatNumberOptions } from "@left-curve/dango/utils";

export type PairStatKind = "priceChange24h" | "volume24h";
export type PairStatTone = "neutral" | "positive" | "negative";

type PairStatValueProps = {
  kind: PairStatKind;
  value: string | null | undefined;
  formatOptions?: Partial<FormatNumberOptions>;
  currency?: string | null;
  className?: string;
  align?: "start" | "end";
  as?: "p" | "span";
};

function asDecimal(value: string | null | undefined) {
  if (value === null || value === undefined) return null;

  try {
    return Decimal(value);
  } catch {
    return null;
  }
}

function formatPairStat(
  kind: PairStatKind,
  value: string | null | undefined,
  formatNumberOptions: FormatNumberOptions,
  formatOptions?: Partial<FormatNumberOptions>,
  currency?: string | null,
): { text: string; tone: PairStatTone } {
  const decimalValue = asDecimal(value);
  if (decimalValue === null) return { text: "-", tone: "neutral" };

  if (kind === "priceChange24h") {
    const maximumTotalDigits = formatOptions?.maximumTotalDigits ?? 6;
    const text = `${decimalValue.gte(0) ? "+" : ""}${formatNumber(value as string, {
      ...formatNumberOptions,
      ...formatOptions,
      maximumTotalDigits,
    })}%`;

    return { text, tone: decimalValue.gte(0) ? "positive" : "negative" };
  }

  const maximumTotalDigits = formatOptions?.maximumTotalDigits ?? 5;
  const selectedCurrency =
    currency === null ? undefined : (currency ?? formatOptions?.currency ?? "usd");
  const text = formatNumber(value as string, {
    ...formatNumberOptions,
    ...formatOptions,
    ...(selectedCurrency ? { currency: selectedCurrency } : {}),
    maximumTotalDigits,
  });

  return { text, tone: "neutral" };
}

export const PairStatValue: React.FC<PairStatValueProps> = ({
  kind,
  value,
  formatOptions,
  currency,
  className,
  align = "start",
  as = "p",
}) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { text, tone } = formatPairStat(kind, value, formatNumberOptions, formatOptions, currency);

  const Component = as;

  return (
    <Component
      className={twMerge(
        "tabular-nums lining-nums",
        align === "end" ? "text-right" : "text-left",
        tone === "positive" && "text-status-success",
        tone === "negative" && "text-status-fail",
        tone === "neutral" && "text-ink-secondary-700",
        className,
      )}
    >
      {text}
    </Component>
  );
};
