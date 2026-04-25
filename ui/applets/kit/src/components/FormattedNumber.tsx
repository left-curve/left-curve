import { useApp } from "@left-curve/foundation";
import { useId } from "react";
import { twMerge } from "@left-curve/foundation";

import { formatDisplayNumber } from "@left-curve/dango/utils";

import type React from "react";
import type { FormatNumberOptions } from "@left-curve/dango/utils";

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

  const parts = formatDisplayNumber(number, { ...formatNumberOptions, ...formatOptions });
  const Component = as;

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
