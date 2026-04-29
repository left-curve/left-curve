import { useApp, twMerge } from "@left-curve/foundation";

import { Decimal } from "@left-curve/dango/utils";
import { FormattedNumber } from "./FormattedNumber";

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

export const PairStatValue: React.FC<PairStatValueProps> = ({
  kind,
  value,
  formatOptions,
  currency,
  className,
  align = "start",
  as = "p",
}) => {
  const decimalValue = asDecimal(value);

  const tone: PairStatTone =
    decimalValue === null
      ? "neutral"
      : kind === "priceChange24h"
        ? decimalValue.gte(0)
          ? "positive"
          : "negative"
        : "neutral";

  const baseClassName = twMerge(
    kind !== "volume24h" && "lining-nums",
    align === "end" ? "text-right" : "text-left",
    tone === "positive" && "text-status-success",
    tone === "negative" && "text-status-fail",
    tone === "neutral" && "text-ink-secondary-700",
    className,
  );

  const Component = as;

  if (decimalValue === null) {
    return <Component className={baseClassName}>-</Component>;
  }

  if (kind === "priceChange24h") {
    return (
      <Component className={baseClassName}>
        {decimalValue.gte(0) ? "+" : ""}
        <FormattedNumber number={value!} formatOptions={formatOptions} as="span" />
        {"%"}
      </Component>
    );
  }

  const selectedCurrency =
    currency === null ? undefined : (currency ?? formatOptions?.currency ?? "usd");

  return (
    <FormattedNumber
      number={value!}
      formatOptions={{
        ...formatOptions,
        ...(selectedCurrency ? { currency: selectedCurrency } : {}),
      }}
      as={as}
      className={baseClassName}
    />
  );
};
