import { useApp } from "@left-curve/foundation";
import { useId } from "react";
import { twMerge } from "@left-curve/foundation";

import { Decimal, formatDisplayNumber } from "@left-curve/utils";

import type React from "react";
import type { FormatNumberOptions } from "@left-curve/utils";

type FormattedNumberProps = {
  number: string | number;
  formatOptions?: Partial<FormatNumberOptions>;
  className?: string;
  as?: "p" | "span";
  tabular?: boolean;
};

export const FormattedNumber: React.FC<FormattedNumberProps> = ({
  number,
  formatOptions,
  className,
  as = "p",
  tabular = false,
}) => {
  const id = useId();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const Component = as;

  if (number === null || number === undefined || Decimal.isNaN(number)) {
    return <Component className={twMerge(className)}>-</Component>;
  }

  const parts = formatDisplayNumber(number, { ...formatNumberOptions, ...formatOptions });

  return (
    <Component className={twMerge(className)}>
      {parts.map((part, index) => {
        if (part.type === "subscript") {
          return (
            <sub
              key={`${id}-part-${
                // biome-ignore lint/suspicious/noArrayIndexKey: parts are positional
                index
              }`}
              className={tabular ? "tabular-nums lining-nums" : "lining-nums"}
            >
              {part.value}
            </sub>
          );
        }
        const isDigit = part.type === "integer" || part.type === "fraction";
        return (
          <span
            key={`${id}-part-${
              // biome-ignore lint/suspicious/noArrayIndexKey: parts are positional
              index
            }`}
            className={isDigit ? (tabular ? "tabular-nums lining-nums" : "lining-nums") : ""}
          >
            {part.value}
          </span>
        );
      })}
    </Component>
  );
};
